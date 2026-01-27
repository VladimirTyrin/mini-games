using System.Collections.Concurrent;
using System.Diagnostics;
using MiniGameNetworkBot.TicTacToe.Bots;
using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe.Training;

public sealed record GameResult(
    bool Bot1PlaysX,
    Mark? Winner,
    Mark Bot1Mark)
{
    public bool Bot1Won => Winner == Bot1Mark;
    public bool Bot2Won => Winner != null && Winner != Bot1Mark;
    public bool IsDraw => Winner == null;
}

public sealed record TournamentResult(
    int Bot1WinsAsX,
    int Bot1WinsAsO,
    int Bot2WinsAsX,
    int Bot2WinsAsO,
    int Draws,
    int TotalGames,
    TimeSpan Duration)
{
    public int Bot1Wins => Bot1WinsAsX + Bot1WinsAsO;
    public int Bot2Wins => Bot2WinsAsX + Bot2WinsAsO;
    public double Bot1WinRate => TotalGames > 0 ? (double)Bot1Wins / TotalGames : 0;
    public double Bot2WinRate => TotalGames > 0 ? (double)Bot2Wins / TotalGames : 0;
    public double DrawRate => TotalGames > 0 ? (double)Draws / TotalGames : 0;
}

public sealed class LocalTournament
{
    private readonly Func<IBot> _bot1Factory;
    private readonly Func<IBot> _bot2Factory;
    private readonly string _bot1Name;
    private readonly string _bot2Name;
    private readonly int _width;
    private readonly int _height;
    private readonly int _winCount;

    public LocalTournament(
        Func<IBot> bot1Factory,
        Func<IBot> bot2Factory,
        string bot1Name,
        string bot2Name,
        int width = 15,
        int height = 15,
        int winCount = 5)
    {
        _bot1Factory = bot1Factory;
        _bot2Factory = bot2Factory;
        _bot1Name = bot1Name;
        _bot2Name = bot2Name;
        _width = width;
        _height = height;
        _winCount = winCount;
    }

    public TournamentResult Run(int totalGames, int maxThreads, CancellationToken ct = default)
    {
        Console.WriteLine($"Tournament: {_bot1Name} vs {_bot2Name}");
        Console.WriteLine($"Games: {totalGames}, Threads: {maxThreads}");
        Console.WriteLine();

        var stopwatch = Stopwatch.StartNew();
        var results = new ConcurrentBag<GameResult>();
        var completedGames = 0;

        var options = new ParallelOptions
        {
            MaxDegreeOfParallelism = maxThreads,
            CancellationToken = ct
        };

        try
        {
            Parallel.For(0, totalGames, options, gameIndex =>
            {
                var result = PlayGame(gameIndex);
                results.Add(result);

                var completed = Interlocked.Increment(ref completedGames);
                if (completed % 10 == 0 || completed == totalGames)
                {
                    var bot1Wins = results.Count(r => r.Bot1Won);
                    var bot2Wins = results.Count(r => r.Bot2Won);
                    var draws = results.Count(r => r.IsDraw);
                    Console.Write($"\rProgress: {completed}/{totalGames} | {_bot1Name}: {bot1Wins} | {_bot2Name}: {bot2Wins} | Draws: {draws}    ");
                }
            });
        }
        catch (OperationCanceledException)
        {
            Console.WriteLine("\nTournament cancelled.");
        }

        stopwatch.Stop();
        Console.WriteLine();

        var bot1WinsAsX = results.Count(r => r.Bot1Won && r.Bot1Mark == Mark.X);
        var bot1WinsAsO = results.Count(r => r.Bot1Won && r.Bot1Mark == Mark.O);
        var bot2WinsAsX = results.Count(r => r.Bot2Won && r.Bot1Mark == Mark.O);
        var bot2WinsAsO = results.Count(r => r.Bot2Won && r.Bot1Mark == Mark.X);
        var draws = results.Count(r => r.IsDraw);

        return new TournamentResult(
            bot1WinsAsX,
            bot1WinsAsO,
            bot2WinsAsX,
            bot2WinsAsO,
            draws,
            results.Count,
            stopwatch.Elapsed);
    }

    private GameResult PlayGame(int gameIndex)
    {
        var bot1 = _bot1Factory();
        var bot2 = _bot2Factory();
        var engine = new GameEngine(_width, _height, _winCount);

        var bot1PlaysX = gameIndex % 2 == 0;
        var bot1Mark = bot1PlaysX ? Mark.X : Mark.O;

        while (!engine.IsGameOver)
        {
            var moves = engine.GetAvailableMoves();
            if (moves.Count == 0)
                break;

            var isBot1Turn = engine.CurrentPlayer == bot1Mark;
            var currentBot = isBot1Turn ? bot1 : bot2;

            var (x, y) = currentBot.GetMove(engine);
            engine.Place(x, y);
        }

        return new GameResult(bot1PlaysX, engine.Winner, bot1Mark);
    }

    public static void PrintResult(TournamentResult result, string bot1Name, string bot2Name)
    {
        var gamesAsX = result.TotalGames / 2;
        var gamesAsO = result.TotalGames - gamesAsX;

        Console.WriteLine();
        Console.WriteLine("=== Tournament Results ===");
        Console.WriteLine($"Total games: {result.TotalGames}");
        Console.WriteLine($"Duration: {result.Duration.TotalSeconds:F1}s ({result.TotalGames / result.Duration.TotalSeconds:F1} games/sec)");
        Console.WriteLine();

        Console.WriteLine("Overall:");
        Console.WriteLine($"  {bot1Name,-18} {result.Bot1Wins,4} wins ({result.Bot1WinRate * 100,5:F1}%)");
        Console.WriteLine($"  {bot2Name,-18} {result.Bot2Wins,4} wins ({result.Bot2WinRate * 100,5:F1}%)");
        Console.WriteLine($"  {"Draws",-18} {result.Draws,4}      ({result.DrawRate * 100,5:F1}%)");
        Console.WriteLine();

        Console.WriteLine("By starting side (X goes first):");
        Console.WriteLine($"  {bot1Name} as X: {result.Bot1WinsAsX}/{gamesAsX} wins ({(gamesAsX > 0 ? result.Bot1WinsAsX * 100.0 / gamesAsX : 0):F1}%)");
        Console.WriteLine($"  {bot1Name} as O: {result.Bot1WinsAsO}/{gamesAsO} wins ({(gamesAsO > 0 ? result.Bot1WinsAsO * 100.0 / gamesAsO : 0):F1}%)");
        Console.WriteLine($"  {bot2Name} as X: {result.Bot2WinsAsX}/{gamesAsO} wins ({(gamesAsO > 0 ? result.Bot2WinsAsX * 100.0 / gamesAsO : 0):F1}%)");
        Console.WriteLine($"  {bot2Name} as O: {result.Bot2WinsAsO}/{gamesAsX} wins ({(gamesAsX > 0 ? result.Bot2WinsAsO * 100.0 / gamesAsX : 0):F1}%)");
        Console.WriteLine();

        if (result.Bot1WinRate > result.Bot2WinRate)
            Console.WriteLine($"Winner: {bot1Name}");
        else if (result.Bot2WinRate > result.Bot1WinRate)
            Console.WriteLine($"Winner: {bot2Name}");
        else
            Console.WriteLine("Result: Tie");
    }
}
