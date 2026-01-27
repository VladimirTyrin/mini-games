using MiniGameNetworkBot.TicTacToe.Core;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Adapters;

public static class ProtoConverter
{
    public static GameEngine ToGameEngine(TicTacToeGameState state)
    {
        var engine = new GameEngine((int)state.FieldWidth, (int)state.FieldHeight, (int)state.WinCount);

        var xMoves = state.Board.Where(c => c.Mark == MarkType.X).ToList();
        var oMoves = state.Board.Where(c => c.Mark == MarkType.O).ToList();

        var maxMoves = Math.Max(xMoves.Count, oMoves.Count);
        for (var i = 0; i < maxMoves; i++)
        {
            if (i < xMoves.Count)
                engine.Place((int)xMoves[i].X, (int)xMoves[i].Y);
            if (i < oMoves.Count)
                engine.Place((int)oMoves[i].X, (int)oMoves[i].Y);
        }

        return engine;
    }
}
