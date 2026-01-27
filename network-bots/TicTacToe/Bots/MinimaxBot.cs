using MiniGameNetworkBot.TicTacToe.LocalGame;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class MinimaxBot : ITicTacToeBot
{
    private readonly ILocalBot _localBot;

    public string Name => "Minimax";

    public MinimaxBot(int maxDepth = 3)
    {
        _localBot = new LocalMinimaxBot(maxDepth);
    }

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var engine = ProtoConverter.ToGameEngine(gameState);
        var (x, y) = _localBot.GetMove(engine);
        return new PlaceMarkCommand { X = (uint)x, Y = (uint)y };
    }
}
