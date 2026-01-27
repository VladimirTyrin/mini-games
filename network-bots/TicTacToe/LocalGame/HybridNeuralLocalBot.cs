namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public sealed class HybridNeuralLocalBot : ILocalBot
{
    private readonly PolicyValueNetwork _network;

    public string Name => "HybridNeural";

    public HybridNeuralLocalBot(PolicyValueNetwork network)
    {
        _network = network;
    }

    public (int X, int Y) GetMove(GameEngine engine)
    {
        return GetMove((IBoardView)engine);
    }

    public (int X, int Y) GetMove(IBoardView board)
    {
        var availableMoves = board.GetAvailableMoves();

        if (availableMoves.Count == 0)
            return (0, 0);

        var tacticalMove = TacticsEngine.GetTacticalMove(board, board.CurrentPlayer);
        if (tacticalMove.HasValue)
            return tacticalMove.Value;

        var state = board.GetBoardStateForNetwork();
        var (policy, _) = _network.Predict(state);

        var bestMove = availableMoves[0];
        var bestProb = float.MinValue;

        foreach (var (x, y) in availableMoves)
        {
            var prob = policy[y * board.Width + x];
            if (prob > bestProb)
            {
                bestProb = prob;
                bestMove = (x, y);
            }
        }

        return bestMove;
    }
}
