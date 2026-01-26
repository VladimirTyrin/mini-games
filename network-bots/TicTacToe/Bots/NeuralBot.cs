using MiniGameNetworkBot.TicTacToe.LocalGame;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class NeuralBot : ITicTacToeBot
{
    private readonly PolicyValueNetwork _network;
    private readonly float _temperature;

    public string Name => "Neural";

    public NeuralBot(string? modelPath = null, float temperature = 0.1f)
    {
        _network = new PolicyValueNetwork();
        _temperature = temperature;

        if (modelPath != null && File.Exists(modelPath))
        {
            Console.WriteLine($"[Neural] Loading model from {modelPath}");
            _network.LoadModel(modelPath);
        }
        else
        {
            Console.WriteLine("[Neural] Using untrained network");
        }

        if (TorchSharp.torch.cuda.is_available())
        {
            Console.WriteLine("[Neural] Using CUDA");
            _network.MoveToDevice(TorchSharp.torch.CUDA);
        }
    }

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var width = (int)gameState.FieldWidth;
        var height = (int)gameState.FieldHeight;

        var boardState = GetBoardState(gameState);
        var availableMoves = GetAvailableMoves(gameState);

        if (availableMoves.Count == 0)
            return new PlaceMarkCommand { X = (uint)(width / 2), Y = (uint)(height / 2) };

        // Check for immediate win
        foreach (var (mx, my) in availableMoves)
        {
            if (IsWinningMove(gameState, mx, my, true))
                return new PlaceMarkCommand { X = (uint)mx, Y = (uint)my };
        }

        // Check for immediate block
        foreach (var (mx, my) in availableMoves)
        {
            if (IsWinningMove(gameState, mx, my, false))
                return new PlaceMarkCommand { X = (uint)mx, Y = (uint)my };
        }

        var (policy, value) = _network.Predict(boardState);

        var bestMove = availableMoves[0];
        var bestScore = float.MinValue;

        foreach (var (x, y) in availableMoves)
        {
            var prob = policy[y * width + x];
            var score = _temperature > 0 ? MathF.Pow(prob, 1f / _temperature) : prob;

            if (score > bestScore)
            {
                bestScore = score;
                bestMove = (x, y);
            }
        }

        return new PlaceMarkCommand { X = (uint)bestMove.X, Y = (uint)bestMove.Y };
    }

    private float[] GetBoardState(TicTacToeGameState gameState)
    {
        var width = (int)gameState.FieldWidth;
        var height = (int)gameState.FieldHeight;
        var state = new float[3 * height * width];

        var isPlayerX = gameState.CurrentPlayer?.PlayerId == gameState.PlayerX?.PlayerId;

        foreach (var cell in gameState.Board)
        {
            var idx = (int)(cell.Y * width + cell.X);
            var isMine = (cell.Mark == MarkType.X) == isPlayerX;

            if (isMine)
                state[idx] = 1f;
            else
                state[height * width + idx] = 1f;
        }

        for (var i = 0; i < height * width; i++)
        {
            if (state[i] == 0 && state[height * width + i] == 0)
                state[2 * height * width + i] = 1f;
        }

        return state;
    }

    private List<(int X, int Y)> GetAvailableMoves(TicTacToeGameState gameState)
    {
        var width = (int)gameState.FieldWidth;
        var height = (int)gameState.FieldHeight;

        if (gameState.Board.Count == 0)
            return [(width / 2, height / 2)];

        var occupied = new HashSet<(int, int)>();
        foreach (var cell in gameState.Board)
            occupied.Add(((int)cell.X, (int)cell.Y));

        var moves = new HashSet<(int, int)>();

        foreach (var cell in gameState.Board)
        {
            for (var dy = -2; dy <= 2; dy++)
            {
                for (var dx = -2; dx <= 2; dx++)
                {
                    var nx = (int)cell.X + dx;
                    var ny = (int)cell.Y + dy;
                    if (nx >= 0 && ny >= 0 && nx < width && ny < height && !occupied.Contains((nx, ny)))
                        moves.Add((nx, ny));
                }
            }
        }

        return moves.ToList();
    }

    private bool IsWinningMove(TicTacToeGameState gameState, int x, int y, bool forSelf)
    {
        var width = (int)gameState.FieldWidth;
        var height = (int)gameState.FieldHeight;
        var winCount = (int)gameState.WinCount;

        var isPlayerX = gameState.CurrentPlayer?.PlayerId == gameState.PlayerX?.PlayerId;
        var targetMark = forSelf
            ? (isPlayerX ? MarkType.X : MarkType.O)
            : (isPlayerX ? MarkType.O : MarkType.X);

        var board = new Dictionary<(int, int), MarkType>();
        foreach (var cell in gameState.Board)
            board[((int)cell.X, (int)cell.Y)] = cell.Mark;

        board[(x, y)] = targetMark;

        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (!board.TryGetValue((nx, ny), out var mark) || mark != targetMark)
                    break;
                count++;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (!board.TryGetValue((nx, ny), out var mark) || mark != targetMark)
                    break;
                count++;
            }

            if (count >= winCount)
                return true;
        }

        return false;
    }
}
