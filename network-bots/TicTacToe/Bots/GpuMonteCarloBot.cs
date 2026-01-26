using ILGPU;
using ILGPU.Runtime;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe.Bots;

public sealed class GpuMonteCarloBot : ITicTacToeBot, IDisposable
{
    private const byte Empty = 1;  // MARK_TYPE_EMPTY = 1
    private const byte MarkX = 2;  // MARK_TYPE_X = 2
    private const byte MarkO = 3;  // MARK_TYPE_O = 3

    private readonly int _simulationsPerMove;
    private readonly Context _context;
    private readonly Accelerator _accelerator;
    private readonly Action<Index1D, ArrayView<byte>, ArrayView<int>, ArrayView<uint>, int, int, int, byte, int> _kernel;

    public GpuMonteCarloBot(int simulationsPerMove = 100000)
    {
        _simulationsPerMove = simulationsPerMove;
        _context = Context.CreateDefault();

        var discreteGpu = _context.Devices
            .Where(d => d.AcceleratorType == AcceleratorType.Cuda || d.AcceleratorType == AcceleratorType.OpenCL)
            .FirstOrDefault(d => !d.Name.Contains("Intel", StringComparison.OrdinalIgnoreCase));

        var device = discreteGpu ?? _context.GetPreferredDevice(preferCPU: false);
        _accelerator = device.CreateAccelerator(_context);
        Console.WriteLine($"[GPU] Using: {_accelerator.Name} ({_accelerator.AcceleratorType})");

        _kernel = _accelerator.LoadAutoGroupedStreamKernel<
            Index1D, ArrayView<byte>, ArrayView<int>, ArrayView<uint>, int, int, int, byte, int>(SimulateKernel);
    }

    public string Name => "GPU";

    public PlaceMarkCommand Move(TicTacToeGameState gameState)
    {
        var width = (int)gameState.FieldWidth;
        var height = (int)gameState.FieldHeight;
        var winCount = (int)gameState.WinCount;

        var board = new byte[height * width];
        foreach (var cell in gameState.Board)
            board[cell.Y * width + cell.X] = (byte)cell.Mark;

        byte botMark = (byte)(gameState.CurrentPlayer?.PlayerId == gameState.PlayerX?.PlayerId ? MarkType.X : MarkType.O);
        byte opponentMark = botMark == MarkX ? MarkO : MarkX;

        var availableMoves = GetAvailableMoves(board, width, height);
        if (availableMoves.Count == 0)
            return new PlaceMarkCommand { X = (uint)(width / 2), Y = (uint)(height / 2) };

        // Check for immediate win
        foreach (var (mx, my) in availableMoves)
        {
            board[my * width + mx] = botMark;
            if (CheckWin(board, width, height, winCount, mx, my))
            {
                return new PlaceMarkCommand { X = (uint)mx, Y = (uint)my };
            }
            board[my * width + mx] = 0;
        }

        // Collect ALL opponent threats
        var opponentThreats = new List<(int X, int Y, int ThreatLevel, bool IsOpen)>();
        foreach (var (mx, my) in availableMoves)
        {
            board[my * width + mx] = opponentMark;
            if (CheckWin(board, width, height, winCount, mx, my))
            {
                opponentThreats.Add((mx, my, 1000, true));
            }
            else
            {
                var (threat, isOpen) = CountOpenThreat(board, width, height, winCount, mx, my, opponentMark);
                if (threat >= winCount - 2)
                    opponentThreats.Add((mx, my, threat + (isOpen ? 10 : 0), isOpen));
            }
            board[my * width + mx] = 0;
        }

        // Must block immediate win threats
        var immediateThreats = opponentThreats.Where(t => t.ThreatLevel >= 1000).ToList();
        if (immediateThreats.Count > 0)
            return new PlaceMarkCommand { X = (uint)immediateThreats[0].X, Y = (uint)immediateThreats[0].Y };

        // Check for creating our own winning double threat (opponent can't block both)
        foreach (var (mx, my) in availableMoves)
        {
            board[my * width + mx] = botMark;
            var threatCount = CountDoubleThreat(board, width, height, winCount, mx, my, botMark);
            board[my * width + mx] = 0;

            if (threatCount >= 2)
                return new PlaceMarkCommand { X = (uint)mx, Y = (uint)my };
        }

        // Block open threats - prioritize by threat level
        // Open four (4 + 10) = must block
        // Closed four (4) = must block
        // Open three (3 + 10) = should block (becomes open four next turn)
        if (opponentThreats.Count > 0)
        {
            var best = opponentThreats.OrderByDescending(t => t.ThreatLevel).First();
            // Block if it's 4+ in a row, or open 3 (threat level 13 = 3 + 10)
            if (best.ThreatLevel >= winCount - 2 + 10 || best.ThreatLevel >= winCount - 1)
                return new PlaceMarkCommand { X = (uint)best.X, Y = (uint)best.Y };
        }

        var bestMove = availableMoves[0];
        var bestScore = int.MinValue;

        var simsPerMove = _simulationsPerMove / availableMoves.Count;

        using var boardBuffer = _accelerator.Allocate1D<byte>(board.Length);
        using var resultsBuffer = _accelerator.Allocate1D<int>(simsPerMove);
        using var seedsBuffer = _accelerator.Allocate1D<uint>(simsPerMove);

        var random = new Random();
        var seeds = new uint[simsPerMove];
        for (var i = 0; i < simsPerMove; i++)
            seeds[i] = (uint)random.Next();

        seedsBuffer.CopyFromCPU(seeds);

        foreach (var (mx, my) in availableMoves)
        {
            var testBoard = (byte[])board.Clone();
            testBoard[my * width + mx] = botMark;

            boardBuffer.CopyFromCPU(testBoard);
            resultsBuffer.MemSetToZero();

            var moveIndex = my * width + mx;
            _kernel(simsPerMove, boardBuffer.View, resultsBuffer.View, seedsBuffer.View,
                width, height, winCount, botMark, moveIndex);

            _accelerator.Synchronize();

            var results = resultsBuffer.GetAsArray1D();
            var wins = results.Sum();

            // Add heuristic bonus for building threats
            var ourThreat = CountThreat(testBoard, width, height, winCount, mx, my, botMark);
            var heuristicBonus = ourThreat * simsPerMove / 10;

            // Bonus for central position
            var centerX = width / 2;
            var centerY = height / 2;
            var distFromCenter = Math.Abs(mx - centerX) + Math.Abs(my - centerY);
            var centralBonus = (width + height - distFromCenter) * simsPerMove / 100;

            var score = wins + heuristicBonus + centralBonus;

            if (score > bestScore)
            {
                bestScore = score;
                bestMove = (mx, my);
            }
        }

        return new PlaceMarkCommand { X = (uint)bestMove.X, Y = (uint)bestMove.Y };
    }

    private static void SimulateKernel(
        Index1D index,
        ArrayView<byte> board,
        ArrayView<int> results,
        ArrayView<uint> seeds,
        int width,
        int height,
        int winCount,
        byte botMark,
        int lastMoveIndex)
    {
        var seed = seeds[index] ^ (uint)index;
        var boardCopy = new byte[225]; // max 15x15
        var size = width * height;

        for (var i = 0; i < size; i++)
            boardCopy[i] = board[i];

        byte currentMark = botMark == 2 ? (byte)3 : (byte)2; // opponent's turn after bot move (X=2, O=3)
        var lastX = lastMoveIndex % width;
        var lastY = lastMoveIndex / width;

        if (CheckWin(boardCopy, width, height, winCount, lastX, lastY))
        {
            results[index] = 1;
            return;
        }

        var moveCount = 0;
        for (var i = 0; i < size; i++)
            if (boardCopy[i] == 0)
                moveCount++;

        while (moveCount > 0)
        {
            seed = XorShift(seed);
            var moveIdx = (int)(seed % (uint)moveCount);

            var count = 0;
            var placed = false;
            for (var i = 0; i < size && !placed; i++)
            {
                if (boardCopy[i] == 0)
                {
                    if (count == moveIdx)
                    {
                        boardCopy[i] = currentMark;
                        lastX = i % width;
                        lastY = i / width;
                        placed = true;
                    }
                    count++;
                }
            }

            if (CheckWin(boardCopy, width, height, winCount, lastX, lastY))
            {
                results[index] = currentMark == botMark ? 1 : 0;
                return;
            }

            currentMark = currentMark == 2 ? (byte)3 : (byte)2; // switch between X(2) and O(3)
            moveCount--;
        }

        results[index] = 0; // draw = 0
    }

    private static uint XorShift(uint state)
    {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        return state;
    }

    private static int CountDoubleThreat(byte[] board, int width, int height, int winCount, int x, int y, byte mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];
        var threatDirections = 0;

        for (var d = 0; d < 4; d++)
        {
            var count = 1;
            var openEnds = 0;

            var canExtendPositive = true;
            for (var i = 1; i < winCount && canExtendPositive; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                {
                    canExtendPositive = false;
                    break;
                }

                var cell = board[ny * width + nx];
                if (cell == mark)
                    count++;
                else if (cell == 0)
                {
                    openEnds++;
                    canExtendPositive = false;
                }
                else
                    canExtendPositive = false;
            }

            var canExtendNegative = true;
            for (var i = 1; i < winCount && canExtendNegative; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                {
                    canExtendNegative = false;
                    break;
                }

                var cell = board[ny * width + nx];
                if (cell == mark)
                    count++;
                else if (cell == 0)
                {
                    openEnds++;
                    canExtendNegative = false;
                }
                else
                    canExtendNegative = false;
            }

            if (count >= winCount - 1 && openEnds >= 1)
                threatDirections++;
        }

        return threatDirections;
    }

    private static (int Count, bool IsOpen) CountOpenThreat(byte[] board, int width, int height, int winCount, int x, int y, byte mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];
        var maxCount = 0;
        var maxIsOpen = false;

        for (var d = 0; d < 4; d++)
        {
            var count = 1;
            var openPositive = false;
            var openNegative = false;

            for (var i = 1; i < winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                var cell = board[ny * width + nx];
                if (cell == mark)
                    count++;
                else if (cell == 0)
                {
                    openPositive = true;
                    break;
                }
                else
                    break;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                var cell = board[ny * width + nx];
                if (cell == mark)
                    count++;
                else if (cell == 0)
                {
                    openNegative = true;
                    break;
                }
                else
                    break;
            }

            var isOpen = openPositive || openNegative;
            if (count > maxCount || (count == maxCount && isOpen && !maxIsOpen))
            {
                maxCount = count;
                maxIsOpen = isOpen;
            }
        }

        return (maxCount, maxIsOpen);
    }

    private static int CountThreat(byte[] board, int width, int height, int winCount, int x, int y, byte mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];
        var maxCount = 0;

        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < winCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (board[ny * width + nx] != mark)
                    break;
                count++;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (board[ny * width + nx] != mark)
                    break;
                count++;
            }

            maxCount = Math.Max(maxCount, count);
        }

        return maxCount;
    }

    private static bool CheckWin(byte[] board, int width, int height, int winCount, int x, int y)
    {
        var mark = board[y * width + x];
        if (mark == 0) return false;

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
                if (board[ny * width + nx] != mark)
                    break;
                count++;
            }

            for (var i = 1; i < winCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= width || ny >= height)
                    break;
                if (board[ny * width + nx] != mark)
                    break;
                count++;
            }

            if (count >= winCount)
                return true;
        }

        return false;
    }

    private static List<(int X, int Y)> GetAvailableMoves(byte[] board, int width, int height)
    {
        var hasAnyMark = false;
        var moves = new HashSet<(int, int)>();

        for (var y = 0; y < height; y++)
        {
            for (var x = 0; x < width; x++)
            {
                if (board[y * width + x] == 0)
                    continue;

                hasAnyMark = true;

                for (var dy = -2; dy <= 2; dy++)
                {
                    for (var dx = -2; dx <= 2; dx++)
                    {
                        var nx = x + dx;
                        var ny = y + dy;
                        if (nx >= 0 && ny >= 0 && nx < width && ny < height && board[ny * width + nx] == 0)
                            moves.Add((nx, ny));
                    }
                }
            }
        }

        if (!hasAnyMark)
            return [(width / 2, height / 2)];

        return moves.ToList();
    }

    public void Dispose()
    {
        _accelerator.Dispose();
        _context.Dispose();
    }
}
