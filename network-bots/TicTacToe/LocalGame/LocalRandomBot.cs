namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public sealed class LocalRandomBot : ILocalBot
{
    private readonly Random _random = new();

    public string Name => "Random";

    public (int X, int Y) GetMove(GameEngine engine)
    {
        var moves = engine.GetAvailableMoves();
        return moves.Count > 0 ? moves[_random.Next(moves.Count)] : (0, 0);
    }
}
