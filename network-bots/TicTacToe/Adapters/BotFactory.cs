using MiniGameNetworkBot.TicTacToe.Bots;
using MiniGameNetworkBot.TicTacToe.Training;

namespace MiniGameNetworkBot.TicTacToe.Adapters;

public static class BotFactory
{
    public static IBot Create(BotType type, string? modelPath = null, int minimaxDepth = 2)
    {
        return type switch
        {
            BotType.Minimax => new MinimaxBot(minimaxDepth),
            BotType.Mcts => new MctsBot(simulationsPerMove: 1000),
            BotType.Hybrid => CreateHybridBot(modelPath),
            BotType.Random => new RandomBot(),
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "Unknown bot type")
        };
    }

    public static Func<IBot> CreateFactory(BotType type, string? modelPath = null, int minimaxDepth = 2)
    {
        if (type == BotType.Hybrid)
        {
            var network = LoadNetwork(modelPath);
            return () => new HybridNeuralBot(network);
        }

        return () => Create(type, modelPath, minimaxDepth);
    }

    public static string GetBotName(BotType type, int minimaxDepth = 2)
    {
        return type switch
        {
            BotType.Minimax => $"Minimax(d={minimaxDepth})",
            BotType.Mcts => "MCTS",
            BotType.Hybrid => "HybridNeural",
            BotType.Random => "Random",
            _ => type.ToString()
        };
    }

    public static Tictactoe.TicTacToeBotType ToServerBotType(OpponentType type)
    {
        return type switch
        {
            OpponentType.Minimax => Tictactoe.TicTacToeBotType.Minimax,
            OpponentType.Random => Tictactoe.TicTacToeBotType.Random,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "Unknown opponent type")
        };
    }

    private static HybridNeuralBot CreateHybridBot(string? modelPath)
    {
        var network = LoadNetwork(modelPath);
        return new HybridNeuralBot(network);
    }

    private static PolicyValueNetwork LoadNetwork(string? modelPath)
    {
        var network = new PolicyValueNetwork();

        if (modelPath != null && File.Exists(modelPath))
        {
            Console.WriteLine($"[HybridNeural] Loading model from {modelPath}");
            network.LoadModel(modelPath);
        }
        else
        {
            Console.WriteLine("[HybridNeural] Using untrained network");
        }

        if (TorchSharp.torch.cuda.is_available())
        {
            Console.WriteLine("[HybridNeural] Using CUDA");
            network.MoveToDevice(TorchSharp.torch.CUDA);
        }

        return network;
    }
}
