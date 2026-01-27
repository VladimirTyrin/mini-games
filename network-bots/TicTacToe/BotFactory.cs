using MiniGameNetworkBot.TicTacToe.Bots;
using MiniGameNetworkBot.TicTacToe.LocalGame;

namespace MiniGameNetworkBot.TicTacToe;

public static class BotFactory
{
    public static ITicTacToeBot Create(BotType type, string? modelPath = null)
    {
        return type switch
        {
            BotType.Minimax => new MinimaxBot(),
            BotType.Mcts => new MctsBot(),
            BotType.Hybrid => CreateHybridBot(modelPath),
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "Unknown bot type")
        };
    }

    private static ITicTacToeBot CreateHybridBot(string? modelPath)
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

        var localBot = new HybridNeuralLocalBot(network);
        return new NetworkBotAdapter(localBot);
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
}
