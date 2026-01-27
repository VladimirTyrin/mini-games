using MiniGameNetworkBot.TicTacToe.LocalGame;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public sealed class NetworkBotAdapter : ITicTacToeBot, IDisposable
{
    private readonly ILocalBot _localBot;
    private readonly IDisposable? _disposable;

    public string Name => _localBot.Name;

    public NetworkBotAdapter(ILocalBot localBot)
    {
        _localBot = localBot;
        _disposable = localBot as IDisposable;
    }

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        (int X, int Y) move;

        if (_localBot is HybridNeuralLocalBot hybridBot)
        {
            var boardView = new ProtoStateView(gameState);
            move = hybridBot.GetMove(boardView);
        }
        else
        {
            var engine = ProtoConverter.ToGameEngine(gameState);
            move = _localBot.GetMove(engine);
        }

        return new PlaceMarkCommand { X = (uint)move.X, Y = (uint)move.Y };
    }

    public void Dispose()
    {
        _disposable?.Dispose();
    }
}
