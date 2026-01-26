namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public interface ILocalBot
{
    string Name { get; }
    (int X, int Y) GetMove(GameEngine engine);
}
