using Minigames.V2;

namespace MiniGameNetworkBot.TicTacToe.Core;

/// <summary>Reads the core's <see cref="TicTacToeView"/> in the C# bots' terms.</summary>
public static class TicTacToeViews
{
    public static Mark[,] Cells(TicTacToeView view)
    {
        var width = (int)view.FieldWidth;
        var height = (int)view.FieldHeight;
        var cells = new Mark[height, width];
        for (var index = 0; index < view.Cells.Count && index < width * height; index++)
            cells[index / width, index % width] = ToMark(view.Cells[index]);
        return cells;
    }

    public static Mark ToMark(TicTacToeMark mark) => mark switch
    {
        TicTacToeMark.X => Mark.X,
        TicTacToeMark.O => Mark.O,
        _ => Mark.Empty
    };

    /// <summary>The mark a seat plays; Empty before the first player is chosen.</summary>
    public static Mark MarkOf(TicTacToeView view, uint slot)
    {
        if (view.HasXSlot && view.XSlot == slot)
            return Mark.X;
        if (view.HasOSlot && view.OSlot == slot)
            return Mark.O;
        return Mark.Empty;
    }

    /// <summary>
    /// The mark to move. After the game ended it is the mark that would move next, so a winner
    /// equal to the current player means the side to move has won, as search code expects.
    /// </summary>
    public static Mark CurrentMark(TicTacToeView view)
    {
        if (view.HasCurrentSlot)
            return MarkOf(view, view.CurrentSlot);
        if (view.LastMove is { } last && view.End is not null)
        {
            var lastMark = ToMark(view.Cells[(int)(last.Y * view.FieldWidth + last.X)]);
            return lastMark == Mark.X ? Mark.O : Mark.X;
        }
        return Mark.X;
    }

    /// <summary>Null while the game runs and after a draw.</summary>
    public static Mark? Winner(TicTacToeView view) =>
        view.End is { HasWinnerSlot: true } end ? MarkOf(view, end.WinnerSlot) : null;
}
