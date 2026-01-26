using MiniGameNetworkBot.TicTacToe.LocalGame;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class NeuralMctsBot : ITicTacToeBot, IDisposable
{
    private readonly PolicyValueNetwork _network;
    private readonly int _simulations;

    public string Name => "NeuralMCTS";

    public NeuralMctsBot(string? modelPath = null, int simulations = 100)
    {
        _network = new PolicyValueNetwork();
        _simulations = simulations;

        if (modelPath != null && File.Exists(modelPath))
        {
            Console.WriteLine($"[NeuralMCTS] Loading model from {modelPath}");
            _network.LoadModel(modelPath);
        }
        else
        {
            Console.WriteLine("[NeuralMCTS] Using untrained network");
        }

        if (TorchSharp.torch.cuda.is_available())
        {
            Console.WriteLine("[NeuralMCTS] Using CUDA");
            _network.MoveToDevice(TorchSharp.torch.CUDA);
        }

        Console.WriteLine($"[NeuralMCTS] Using {_simulations} simulations per move");
    }

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var engine = ConvertToEngine(gameState);
        var mcts = new NeuralMcts(_network, _simulations);

        var (x, y) = mcts.GetBestMove(engine, temperature: 0.1f);
        return new PlaceMarkCommand { X = (uint)x, Y = (uint)y };
    }

    private static GameEngine ConvertToEngine(TicTacToeGameState state)
    {
        var width = (int)state.FieldWidth;
        var height = (int)state.FieldHeight;
        var winCount = (int)state.WinCount;

        var engine = new GameEngine(width, height, winCount);

        // Determine move order from board state
        var xCount = state.Board.Count(c => c.Mark == MarkType.X);
        var oCount = state.Board.Count(c => c.Mark == MarkType.O);

        // Replay moves to reconstruct engine state
        // X always goes first, so reconstruct alternating
        var xMoves = state.Board.Where(c => c.Mark == MarkType.X).ToList();
        var oMoves = state.Board.Where(c => c.Mark == MarkType.O).ToList();

        for (var i = 0; i < Math.Max(xMoves.Count, oMoves.Count); i++)
        {
            if (i < xMoves.Count)
                engine.Place((int)xMoves[i].X, (int)xMoves[i].Y);
            if (i < oMoves.Count)
                engine.Place((int)oMoves[i].X, (int)oMoves[i].Y);
        }

        return engine;
    }

    public void Dispose()
    {
        _network.Dispose();
    }
}
