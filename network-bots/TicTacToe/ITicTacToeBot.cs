using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public interface ITicTacToeBot
{
    string Name { get; }
    PlaceMarkCommand Move(TicTacToeGameState gameState);
}
