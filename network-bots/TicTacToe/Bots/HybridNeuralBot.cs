using MiniGameNetworkBot.TicTacToe.LocalGame;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class HybridNeuralBot : ITicTacToeBot, IDisposable
{
    private readonly PolicyValueNetwork _network;
    private readonly int _boardWidth;
    private readonly int _boardHeight;
    private readonly int _winCount;

    public string Name => "HybridNeural";

    public HybridNeuralBot(string? modelPath = null, int width = 15, int height = 15, int winCount = 5)
    {
        _network = new PolicyValueNetwork();
        _boardWidth = width;
        _boardHeight = height;
        _winCount = winCount;

        if (modelPath != null && File.Exists(modelPath))
        {
            Console.WriteLine($"[HybridNeural] Loading model from {modelPath}");
            _network.LoadModel(modelPath);
        }
        else
        {
            Console.WriteLine("[HybridNeural] Using untrained network");
        }

        if (TorchSharp.torch.cuda.is_available())
        {
            Console.WriteLine("[HybridNeural] Using CUDA");
            _network.MoveToDevice(TorchSharp.torch.CUDA);
        }
    }

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var board = ConvertBoard(gameState);
        var myMark = GetCurrentMark(gameState);
        var oppMark = myMark == Mark.X ? Mark.O : Mark.X;
        var availableMoves = GetAvailableMoves(board);

        if (availableMoves.Count == 0)
            return new PlaceMarkCommand { X = 0, Y = 0 };

        // 1. Check for immediate win
        foreach (var (x, y) in availableMoves)
        {
            if (IsWinningMove(board, x, y, myMark))
                return new PlaceMarkCommand { X = (uint)x, Y = (uint)y };
        }

        // 2. Block opponent's immediate win
        foreach (var (x, y) in availableMoves)
        {
            if (IsWinningMove(board, x, y, oppMark))
                return new PlaceMarkCommand { X = (uint)x, Y = (uint)y };
        }

        // 3. Create open threat (4 in a row with both ends open)
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenThreat(board, x, y, myMark))
                return new PlaceMarkCommand { X = (uint)x, Y = (uint)y };
        }

        // 4. Block opponent's open threat
        foreach (var (x, y) in availableMoves)
        {
            if (CreatesOpenThreat(board, x, y, oppMark))
                return new PlaceMarkCommand { X = (uint)x, Y = (uint)y };
        }

        // 5. Use neural network for strategic move
        var state = GetBoardState(board, myMark);
        var (policy, _) = _network.Predict(state);

        var bestMove = availableMoves[0];
        var bestProb = float.MinValue;

        foreach (var (x, y) in availableMoves)
        {
            var prob = policy[y * _boardWidth + x];
            if (prob > bestProb)
            {
                bestProb = prob;
                bestMove = (x, y);
            }
        }

        return new PlaceMarkCommand { X = (uint)bestMove.x, Y = (uint)bestMove.y };
    }

    private Mark[,] ConvertBoard(TicTacToeGameState state)
    {
        var board = new Mark[_boardHeight, _boardWidth];

        foreach (var cell in state.Board)
        {
            var mark = cell.Mark switch
            {
                MarkType.X => Mark.X,
                MarkType.O => Mark.O,
                _ => Mark.Empty
            };
            board[cell.Y, cell.X] = mark;
        }

        return board;
    }

    private Mark GetCurrentMark(TicTacToeGameState state)
    {
        var xCount = state.Board.Count(c => c.Mark == MarkType.X);
        var oCount = state.Board.Count(c => c.Mark == MarkType.O);
        return xCount <= oCount ? Mark.X : Mark.O;
    }

    private List<(int x, int y)> GetAvailableMoves(Mark[,] board)
    {
        var moves = new List<(int x, int y)>();
        for (var y = 0; y < _boardHeight; y++)
        {
            for (var x = 0; x < _boardWidth; x++)
            {
                if (board[y, x] == Mark.Empty)
                    moves.Add((x, y));
            }
        }
        return moves;
    }

    private bool IsWinningMove(Mark[,] board, int x, int y, Mark mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < _winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= _boardWidth || ny >= _boardHeight)
                    break;
                if (board[ny, nx] != mark)
                    break;
                count++;
            }

            for (var i = 1; i < _winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= _boardWidth || ny >= _boardHeight)
                    break;
                if (board[ny, nx] != mark)
                    break;
                count++;
            }

            if (count >= _winCount)
                return true;
        }

        return false;
    }

    private bool CreatesOpenThreat(Mark[,] board, int x, int y, Mark mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var d = 0; d < 4; d++)
        {
            var count = 1;
            var openEnds = 0;

            // Count in positive direction
            var posEnd = 1;
            for (var i = 1; i < _winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= _boardWidth || ny >= _boardHeight)
                    break;
                if (board[ny, nx] != mark)
                    break;
                count++;
                posEnd = i + 1;
            }

            // Check if positive end is open
            var checkX = x + dx[d] * posEnd;
            var checkY = y + dy[d] * posEnd;
            if (checkX >= 0 && checkY >= 0 && checkX < _boardWidth && checkY < _boardHeight &&
                board[checkY, checkX] == Mark.Empty)
            {
                openEnds++;
            }

            // Count in negative direction
            var negEnd = 1;
            for (var i = 1; i < _winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= _boardWidth || ny >= _boardHeight)
                    break;
                if (board[ny, nx] != mark)
                    break;
                count++;
                negEnd = i + 1;
            }

            // Check if negative end is open
            checkX = x - dx[d] * negEnd;
            checkY = y - dy[d] * negEnd;
            if (checkX >= 0 && checkY >= 0 && checkX < _boardWidth && checkY < _boardHeight &&
                board[checkY, checkX] == Mark.Empty)
            {
                openEnds++;
            }

            // Open threat: 4 in a row with both ends open
            if (count >= _winCount - 1 && openEnds >= 2)
                return true;
        }

        return false;
    }

    private float[] GetBoardState(Mark[,] board, Mark currentPlayer)
    {
        var state = new float[3 * _boardHeight * _boardWidth];
        var opponent = currentPlayer == Mark.X ? Mark.O : Mark.X;

        for (var y = 0; y < _boardHeight; y++)
        {
            for (var x = 0; x < _boardWidth; x++)
            {
                var idx = y * _boardWidth + x;
                var cell = board[y, x];

                if (cell == currentPlayer)
                    state[idx] = 1f;
                else if (cell == opponent)
                    state[_boardHeight * _boardWidth + idx] = 1f;
                else
                    state[2 * _boardHeight * _boardWidth + idx] = 1f;
            }
        }

        return state;
    }

    public void Dispose()
    {
        _network.Dispose();
    }
}
