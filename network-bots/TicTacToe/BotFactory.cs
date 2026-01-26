using MiniGameNetworkBot.TicTacToe.Bots;

namespace MiniGameNetworkBot.TicTacToe;

public static class BotFactory
{
    public static ITicTacToeBot Create(BotType type, string? modelPath = null)
    {
        return type switch
        {
            BotType.Minimax => new MinimaxBot(),
            BotType.Mcts => new MctsBot(),
            BotType.Gpu => new GpuMonteCarloBot(),
            BotType.Neural => new NeuralBot(modelPath),
            BotType.NeuralMcts => new NeuralMctsBot(modelPath),
            BotType.Hybrid => new HybridNeuralBot(modelPath),
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "Unknown bot type")
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
}
