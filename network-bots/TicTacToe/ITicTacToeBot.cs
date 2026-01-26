using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public interface ITicTacToeBot
{
    PlaceMarkCommand Move(TicTacToeGameState gameState);
}