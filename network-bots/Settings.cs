namespace MiniGameNetworkBot;

public enum BotType
{
    Minimax,
    Mcts,
    Hybrid,
    Random
}

public enum OpponentType
{
    Minimax,
    Random
}

public enum RunMode
{
    Play,
    Train,
    Benchmark
}

public sealed record Settings
{
    public const string ProductionServer = "wss://braintvsminigames.xyz/ws";
    public const string LocalServer = "ws://127.0.0.1:5000/ws";

    public string ServerUrl { get; init; } = ProductionServer;
    public string? BotKey { get; init; } = Environment.GetEnvironmentVariable("MG_BOT_KEY");
    public bool ShowUi { get; init; } = true;
    public BotType BotType { get; init; } = BotType.Minimax;
    public OpponentType OpponentType { get; init; } = OpponentType.Minimax;
    public RunMode Mode { get; init; } = RunMode.Play;
    public string ModelPath { get; init; } = "neural_model.dat";
    public int TrainIterations { get; init; } = 50;
    public int GamesPerIteration { get; init; } = 100;

    public BotType Bot1Type { get; init; } = BotType.Hybrid;
    public BotType Bot2Type { get; init; } = BotType.Minimax;
    public int BenchmarkGames { get; init; } = 100;
    public int BenchmarkThreads { get; init; } = Environment.ProcessorCount;
    public int MinimaxDepth { get; init; } = 2;
    public int BoardSize { get; init; } = 15;
    public int WinCount { get; init; } = 5;

    public static Settings Parse(string[] args)
    {
        var settings = new Settings();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i].ToLowerInvariant();

            switch (arg)
            {
                case "--local":
                    settings = settings with { ServerUrl = LocalServer };
                    break;

                case "--server":
                    if (i + 1 < args.Length)
                        settings = settings with { ServerUrl = args[++i] };
                    break;

                case "--bot-key":
                    if (i + 1 < args.Length)
                        settings = settings with { BotKey = args[++i] };
                    break;

                case "--no-ui":
                    settings = settings with { ShowUi = false };
                    break;

                case "--bot-type" or "-b":
                    if (i + 1 < args.Length)
                    {
                        if (!Enum.TryParse<BotType>(args[++i], ignoreCase: true, out var botType))
                            throw new ArgumentException($"Invalid bot type: {args[i]}. Valid types are: {string.Join(", ", Enum.GetValues<BotType>())}");

                        settings = settings with { BotType = botType };
                    }

                    break;

                case "--opponent" or "-o":
                    if (i + 1 < args.Length && Enum.TryParse<OpponentType>(args[++i], ignoreCase: true, out var oppType))
                        settings = settings with { OpponentType = oppType };
                    break;

                case "--train":
                    settings = settings with { Mode = RunMode.Train };
                    break;

                case "--benchmark":
                    settings = settings with { Mode = RunMode.Benchmark };
                    break;

                case "--bot1":
                    if (i + 1 < args.Length && Enum.TryParse<BotType>(args[++i], ignoreCase: true, out var bot1))
                        settings = settings with { Bot1Type = bot1 };
                    break;

                case "--bot2":
                    if (i + 1 < args.Length && Enum.TryParse<BotType>(args[++i], ignoreCase: true, out var bot2))
                        settings = settings with { Bot2Type = bot2 };
                    break;

                case "--games" or "-g":
                    if (i + 1 < args.Length && int.TryParse(args[++i], out var games))
                        settings = settings with { BenchmarkGames = games };
                    break;

                case "--threads" or "-t":
                    if (i + 1 < args.Length && int.TryParse(args[++i], out var threads))
                        settings = settings with { BenchmarkThreads = threads };
                    break;

                case "--minimax-depth" or "-d":
                    if (i + 1 < args.Length && int.TryParse(args[++i], out var depth))
                        settings = settings with { MinimaxDepth = depth };
                    break;

                case "--board-size":
                    if (i + 1 < args.Length && int.TryParse(args[++i], out var size))
                        settings = settings with { BoardSize = size };
                    break;

                case "--win-count":
                    if (i + 1 < args.Length && int.TryParse(args[++i], out var winCount))
                        settings = settings with { WinCount = winCount };
                    break;

                case "--model":
                    if (i + 1 < args.Length)
                        settings = settings with { ModelPath = args[++i] };
                    break;

                case "--iterations":
                    if (i + 1 < args.Length && int.TryParse(args[++i], out var iterations))
                        settings = settings with { TrainIterations = iterations };
                    break;

                case "--help" or "-h":
                    PrintHelp();
                    Environment.Exit(0);
                    break;
            }
        }

        return settings;
    }

    private static void PrintHelp()
    {
        Console.WriteLine($"""
            TicTacToe Bot

            Usage: dotnet run -- [options]

            Modes:
              (default)               Play against a server bot (protocol v2 over WebSocket)
              --train                 Run self-play training
              --benchmark             Run a local bot vs bot tournament on the Rust core

            Play options:
              --server URL            Server WebSocket (default: {ProductionServer})
              --local                 Same as --server {LocalServer}
              --bot-key KEY           Bot key from the server operator (or MG_BOT_KEY)
              --no-ui                 Disable UI window
              --bot-type, -b TYPE     Bot type: Minimax, Mcts, Hybrid, Random (default: Minimax)
              --opponent, -o TYPE     Server bot: Minimax, Random (default: Minimax)

            Training options:
              --iterations N          Training iterations (default: 50)

            Benchmark options:
              --bot1 TYPE             First bot (default: Hybrid)
              --bot2 TYPE             Second bot (default: Minimax)
              --games, -g N           Number of games (default: 100)
              --threads, -t N         Parallel threads (default: CPU count)
              --minimax-depth, -d N   Minimax search depth (default: 2)
              --board-size N          Board side (default: 15)
              --win-count N           Marks in a row to win (default: 5)

            Common options:
              --model PATH            Neural model path (default: neural_model.dat)
              --help, -h              Show this help

            Examples:
              dotnet run -- --local --bot-key secret --bot-type hybrid --model neural_model.dat
              dotnet run -- --train --iterations 100
              dotnet run -- --benchmark --bot1 minimax --bot2 random --games 100
            """);
    }
}
