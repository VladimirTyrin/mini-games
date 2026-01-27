namespace MiniGameNetworkBot;

public enum BotType
{
    Minimax,
    Mcts,
    Hybrid
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
    public string ServerAddress { get; init; } = "https://braintvsminigames.xyz:5443";
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

    public static Settings Parse(string[] args)
    {
        var settings = new Settings();

        for (var i = 0; i < args.Length; i++)
        {
            var arg = args[i].ToLowerInvariant();

            switch (arg)
            {
                case "--local":
                    settings = settings with { ServerAddress = "http://localhost:5001" };
                    break;

                case "--no-ui":
                    settings = settings with { ShowUi = false };
                    break;

                case "--bot-type" or "-b":
                    if (i + 1 < args.Length && Enum.TryParse<BotType>(args[++i], ignoreCase: true, out var botType))
                        settings = settings with { BotType = botType };
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
        Console.WriteLine("""
            TicTacToe Bot

            Usage: dotnet run -- [options]

            Modes:
              (default)               Play against server
              --train                 Run self-play training
              --benchmark             Run local bot vs bot tournament

            Play options:
              --local                 Connect to localhost:5001
              --no-ui                 Disable UI window
              --bot-type, -b TYPE     Bot type: Minimax, Mcts, Hybrid (default: Minimax)
              --opponent, -o TYPE     Server opponent: Minimax, Random (default: Minimax)

            Training options:
              --iterations N          Training iterations (default: 50)

            Benchmark options:
              --bot1 TYPE             First bot (default: Hybrid)
              --bot2 TYPE             Second bot (default: Minimax)
              --games, -g N           Number of games (default: 100)
              --threads, -t N         Parallel threads (default: CPU count)
              --minimax-depth, -d N   Minimax search depth (default: 2)

            Common options:
              --model PATH            Neural model path (default: neural_model.dat)
              --help, -h              Show this help

            Examples:
              dotnet run -- --local --bot-type hybrid --model neural_model.dat
              dotnet run -- --train --iterations 100
              dotnet run -- --benchmark --bot1 hybrid --bot2 minimax --games 100
            """);
    }
}
