using GameService;
using MiniGameNetworkBot.TicTacToe.Adapters;
using MiniGameNetworkBot.TicTacToe.Bots;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public sealed class TicTacToeRunner
{
    private readonly GameNetworkHandler _networkHandler;
    private readonly IBot _bot;
    private readonly TicTacToeBotType _opponentType;

    public TicTacToeRunner(GameNetworkHandler networkHandler, IBot bot, TicTacToeBotType opponentType = TicTacToeBotType.Minimax)
    {
        _networkHandler = networkHandler;
        _bot = bot;
        _opponentType = opponentType;
    }

    public async Task<bool> RunAsync(CancellationToken cancellationToken)
    {
        await _networkHandler.EnqueueSendAsync(new ClientMessage
        {
            CreateLobby = new CreateLobbyRequest
            {
                MaxPlayers = 2,
                LobbyName = "Bot game " + Guid.NewGuid(),
                Settings = new LobbySettings
                {
                    Tictactoe = new TicTacToeLobbySettings
                    {
                        FieldHeight = 15,
                        FieldWidth = 15,
                        FirstPlayer = FirstPlayerMode.Random,
                        WinCount = 5
                    }
                }
            }
        }, cancellationToken);

        await _networkHandler.EnqueueSendAsync(new ClientMessage
        {
            AddBot = new AddBotRequest
            {
                TictactoeBot = _opponentType
            }
        }, cancellationToken);

        var allMessagesEnumerable = _networkHandler.ReadAllFromThisMomentAsync(cancellationToken);
        await _networkHandler.EnqueueSendAsync(new ClientMessage
        {
            StartGame = new StartGameRequest()
        }, cancellationToken);

        try
        {
            await foreach (var message in allMessagesEnumerable)
            {
                if (message.GameOver is { } gameOver)
                {
                    return gameOver.Winner?.PlayerId == _networkHandler.ClientId;
                }

                if (message.GameState is { Tictactoe: { } tictactoeState })
                {
                    if (tictactoeState.CurrentPlayer?.PlayerId != _networkHandler.ClientId)
                    {
                        continue;
                    }

                    var boardView = new ProtoStateView(tictactoeState);
                    var (x, y) = _bot.GetMove(boardView);

                    await _networkHandler.EnqueueSendAsync(new ClientMessage
                    {
                        InGame = new InGameCommand
                        {
                            Tictactoe = new TicTacToeInGameCommand
                            {
                                Place = new PlaceMarkCommand { X = (uint)x, Y = (uint)y }
                            }
                        }
                    }, cancellationToken);
                }
            }
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            return false;
        }

        throw new Exception("Unexpected end of game messages");
    }
}
