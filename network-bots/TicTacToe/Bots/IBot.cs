using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public interface IBot
{
    string Name { get; }
    (int X, int Y) GetMove(IBoardView board);
}
