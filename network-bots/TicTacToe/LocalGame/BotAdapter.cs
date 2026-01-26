using Tictactoe;
using PlayerIdentity = Tictactoe.PlayerIdentity;

namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public sealed class BotAdapter : ILocalBot
{
    private readonly ITicTacToeBot _bot;

    public string Name => _bot.Name;

    public BotAdapter(ITicTacToeBot bot)
    {
        _bot = bot;
    }

    public (int X, int Y) GetMove(GameEngine engine)
    {
        var state = ConvertToProtoState(engine);
        var command = _bot.Move(state);
        return ((int)command.X, (int)command.Y);
    }

    private static TicTacToeGameState ConvertToProtoState(GameEngine engine)
    {
        var state = new TicTacToeGameState
        {
            FieldWidth = (uint)engine.Width,
            FieldHeight = (uint)engine.Height,
            WinCount = (uint)engine.WinCount,
            Status = engine.IsGameOver ? GameStatus.Draw : GameStatus.InProgress,
            PlayerX = new PlayerIdentity { PlayerId = "X" },
            PlayerO = new PlayerIdentity { PlayerId = "O" },
            CurrentPlayer = engine.CurrentPlayer == Mark.X
                ? new PlayerIdentity { PlayerId = "X" }
                : new PlayerIdentity { PlayerId = "O" }
        };

        for (var y = 0; y < engine.Height; y++)
        {
            for (var x = 0; x < engine.Width; x++)
            {
                var cell = engine.GetCell(x, y);
                if (cell != Mark.Empty)
                {
                    state.Board.Add(new CellMark()
                    {
                        X = (uint)x,
                        Y = (uint)y,
                        Mark = cell == Mark.X ? MarkType.X : MarkType.O
                    });
                }
            }
        }

        return state;
    }
}
