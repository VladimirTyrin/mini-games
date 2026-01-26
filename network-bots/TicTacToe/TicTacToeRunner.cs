using GameService;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public sealed class TicTacToeRunner
{
    private readonly GameNetworkHandler _networkHandler;
    private readonly ITicTacToeBot _bot;

    public TicTacToeRunner(GameNetworkHandler networkHandler, ITicTacToeBot bot)
    {
        _networkHandler = networkHandler;
        _bot = bot;
    }
    
    /// <summary>
    ///     true - won, false - lost, exception - error
    /// </summary>
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
                        FirstPlayer = FirstPlayerMode.Host,
                        WinCount = 5
                    }
                }
            }
        }, cancellationToken);
        
        await _networkHandler.EnqueueSendAsync(new ClientMessage
        {
            AddBot = new AddBotRequest
            {
                TictactoeBot = TicTacToeBotType.Minimax
            }
        }, cancellationToken);
        
        var allMessagesEnumerable = _networkHandler.ReadAllFromThisMomentAsync(cancellationToken);
        await _networkHandler.EnqueueSendAsync(new ClientMessage
        {
            StartGame = new StartGameRequest()
        }, cancellationToken);
        
        await foreach (var message in allMessagesEnumerable.WithCancellation(cancellationToken))
        {
            if (message.GameOver is { } gameOver)
            {
                return gameOver.Winner?.PlayerId == _networkHandler.ClientId;
            }
            
            if (message.GameState is { Tictactoe: {} tictactoeState })
            {
                if (tictactoeState.CurrentPlayer?.PlayerId != _networkHandler.ClientId)
                {
                    continue;
                }

                var botMove = _bot.Move(tictactoeState);

                await _networkHandler.EnqueueSendAsync(new ClientMessage
                {
                    InGame = new InGameCommand
                    {
                        Tictactoe = new TicTacToeInGameCommand
                        {
                            Place = botMove
                        }
                    }
                }, cancellationToken);
            }
        }
        
        throw new Exception("Unexpected end of game messages");
    }
}