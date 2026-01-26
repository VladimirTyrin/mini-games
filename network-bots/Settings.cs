namespace MiniGameNetworkBot;

public enum BotType
{
    Minimax,
    Mcts,
    Gpu,
    Neural,
    NeuralMcts,
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
    Train
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

            Options:
              --local                 Connect to localhost:5001
              --no-ui                 Disable UI window
              --bot-type, -b TYPE     Bot type: Minimax, Mcts, Gpu, Neural, NeuralMcts, Hybrid (default: Minimax)
              --opponent, -o TYPE     Server opponent: Minimax, Random (default: Minimax)
              --train                 Run self-play training for Neural bot
              --model PATH            Neural model path (default: neural_model.dat)
              --iterations N          Training iterations (default: 50)
              --help, -h              Show this help

            Examples:
              dotnet run -- --local --bot-type gpu --no-ui
              dotnet run -- --train --iterations 100
              dotnet run -- --bot-type neural --opponent random
            """);
    }
}
