namespace MiniGameNetworkBot.TicTacToe.Core;

public interface IBoardView
{
    int Width { get; }
    int Height { get; }
    int WinCount { get; }
    Mark CurrentPlayer { get; }
    Mark GetCell(int x, int y);
    List<(int X, int Y)> GetAvailableMoves();
    Mark[,] ToArray();
}
