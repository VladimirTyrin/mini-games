using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class RandomBot : IBot
{
    private readonly Random _random = new();

    public string Name => "Random";

    public (int X, int Y) GetMove(IBoardView board)
    {
        var moves = board.GetAvailableMoves();
        return moves.Count > 0 ? moves[_random.Next(moves.Count)] : (0, 0);
    }
}
