using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Media;
using Avalonia.Threading;
using GameService;
using Tictactoe;

namespace MiniGameNetworkBot.TicTacToe;

public partial class GameWindow : Window
{
    private readonly GameNetworkHandler? _networkHandler;
    private readonly CancellationTokenSource _cts = new();
    private readonly string _botClientId = string.Empty;

    private TicTacToeGameState? _lastState;
    private WinningLine? _winningLine;

    private Canvas _boardCanvas = null!;
    private TextBlock _statusText = null!;
    private TextBlock _botMarkText = null!;
    private TextBlock _opponentMarkText = null!;
    private TextBlock _moveCountText = null!;
    private TextBlock _winConditionText = null!;
    private TextBlock _boardSizeText = null!;

    [Obsolete("Designer only", true)]
    public GameWindow()
    {
        InitializeComponent();
    }

    public GameWindow(GameNetworkHandler networkHandler)
    {
        _networkHandler = networkHandler;
        _botClientId = networkHandler.ClientId;

        InitializeComponent();

        _boardCanvas.SizeChanged += (_, _) => RedrawBoard();
        Closed += (_, _) => _cts.Cancel();

        StartListening();
    }

    private void InitializeComponent()
    {
        Avalonia.Markup.Xaml.AvaloniaXamlLoader.Load(this);

        _boardCanvas = this.FindControl<Canvas>("BoardCanvas")!;
        _statusText = this.FindControl<TextBlock>("StatusText")!;
        _botMarkText = this.FindControl<TextBlock>("BotMarkText")!;
        _opponentMarkText = this.FindControl<TextBlock>("OpponentMarkText")!;
        _moveCountText = this.FindControl<TextBlock>("MoveCountText")!;
        _winConditionText = this.FindControl<TextBlock>("WinConditionText")!;
        _boardSizeText = this.FindControl<TextBlock>("BoardSizeText")!;
    }

    private async void StartListening()
    {
        if (_networkHandler is null) return;

        try
        {
            await foreach (var message in _networkHandler.ReadAllFromThisMomentAsync(_cts.Token))
            {
                HandleMessage(message);
            }
        }
        catch (OperationCanceledException)
        {
        }
    }

    private void HandleMessage(ServerMessage message)
    {
        Dispatcher.UIThread.Post(() =>
        {
            if (message.GameState is { Tictactoe: { } state })
            {
                UpdateState(state);
            }
            else if (message.GameOver is { } gameOver)
            {
                var won = gameOver.Winner?.PlayerId == _botClientId;
                _winningLine = gameOver.TictactoeInfo?.WinningLine;
                ShowGameOver(won);
                RedrawBoard();
            }
            else if (message.GameStarting is not null)
            {
                _winningLine = null;
                SetStatus("Game starting...", false);
            }
        });
    }

    private void UpdateState(TicTacToeGameState state)
    {
        _lastState = state;

        var botMark = GetBotMark(state);
        var opponentMark = botMark == 'X' ? 'O' : 'X';

        _botMarkText.Text = botMark.ToString();
        _opponentMarkText.Text = opponentMark.ToString();
        _moveCountText.Text = $"Move: {state.Board.Count}";
        _winConditionText.Text = $"Win: {state.WinCount} in a row";
        _boardSizeText.Text = $"Board: {state.FieldWidth}x{state.FieldHeight}";

        var isOurTurn = state.CurrentPlayer?.PlayerId == _botClientId;
        SetStatus(isOurTurn ? "Our turn - thinking..." : "Opponent's turn", isOurTurn);

        RedrawBoard();
    }

    private void SetStatus(string status, bool isOurTurn)
    {
        _statusText.Text = status;
        _statusText.Foreground = isOurTurn
            ? new SolidColorBrush(Color.Parse("#a6e3a1"))
            : new SolidColorBrush(Color.Parse("#f9e2af"));
    }

    private void ShowGameOver(bool won)
    {
        _statusText.Text = won ? "WE WON!" : "We lost";
        _statusText.Foreground = won
            ? new SolidColorBrush(Color.Parse("#a6e3a1"))
            : new SolidColorBrush(Color.Parse("#f38ba8"));
        _statusText.FontSize = 24;
    }

    private void RedrawBoard()
    {
        if (_lastState == null) return;

        var state = _lastState;
        _boardCanvas.Children.Clear();

        var width = (int)state.FieldWidth;
        var height = (int)state.FieldHeight;

        var canvasWidth = _boardCanvas.Bounds.Width;
        var canvasHeight = _boardCanvas.Bounds.Height;

        if (canvasWidth <= 0 || canvasHeight <= 0) return;

        var cellSize = Math.Min(canvasWidth / width, canvasHeight / height);
        var offsetX = (canvasWidth - cellSize * width) / 2;
        var offsetY = (canvasHeight - cellSize * height) / 2;

        DrawGrid(width, height, cellSize, offsetX, offsetY);
        DrawMarks(state, cellSize, offsetX, offsetY);

        if (_winningLine is not null)
        {
            DrawWinningLine(_winningLine, cellSize, offsetX, offsetY);
        }
    }

    private void DrawGrid(int width, int height, double cellSize, double offsetX, double offsetY)
    {
        var gridColor = new SolidColorBrush(Color.Parse("#45475a"));

        for (var i = 0; i <= width; i++)
        {
            var line = new Line
            {
                StartPoint = new Point(offsetX + i * cellSize, offsetY),
                EndPoint = new Point(offsetX + i * cellSize, offsetY + height * cellSize),
                Stroke = gridColor,
                StrokeThickness = 1
            };
            _boardCanvas.Children.Add(line);
        }

        for (var i = 0; i <= height; i++)
        {
            var line = new Line
            {
                StartPoint = new Point(offsetX, offsetY + i * cellSize),
                EndPoint = new Point(offsetX + width * cellSize, offsetY + i * cellSize),
                Stroke = gridColor,
                StrokeThickness = 1
            };
            _boardCanvas.Children.Add(line);
        }
    }

    private void DrawMarks(TicTacToeGameState state, double cellSize, double offsetX, double offsetY)
    {
        var botMark = GetBotMark(state);
        var lastMove = state.LastMove;

        foreach (var cell in state.Board)
        {
            var x = (int)cell.X;
            var y = (int)cell.Y;
            var isLast = lastMove != null && lastMove.X == x && lastMove.Y == y;

            var centerX = offsetX + x * cellSize + cellSize / 2;
            var centerY = offsetY + y * cellSize + cellSize / 2;
            var markSize = cellSize * 0.6;

            if (isLast)
            {
                var highlight = new Rectangle
                {
                    Width = cellSize - 2,
                    Height = cellSize - 2,
                    Fill = new SolidColorBrush(Color.Parse("#313244"))
                };
                Canvas.SetLeft(highlight, offsetX + x * cellSize + 1);
                Canvas.SetTop(highlight, offsetY + y * cellSize + 1);
                _boardCanvas.Children.Add(highlight);
            }

            if (cell.Mark == MarkType.X)
            {
                DrawX(centerX, centerY, markSize, botMark == 'X');
            }
            else if (cell.Mark == MarkType.O)
            {
                DrawO(centerX, centerY, markSize, botMark == 'O');
            }
        }
    }

    private void DrawX(double centerX, double centerY, double size, bool isBotMark)
    {
        var color = new SolidColorBrush(isBotMark ? Color.Parse("#89b4fa") : Color.Parse("#f38ba8"));

        var line1 = new Line
        {
            StartPoint = new Point(centerX - size / 2, centerY - size / 2),
            EndPoint = new Point(centerX + size / 2, centerY + size / 2),
            Stroke = color,
            StrokeThickness = 3,
            StrokeLineCap = PenLineCap.Round
        };

        var line2 = new Line
        {
            StartPoint = new Point(centerX + size / 2, centerY - size / 2),
            EndPoint = new Point(centerX - size / 2, centerY + size / 2),
            Stroke = color,
            StrokeThickness = 3,
            StrokeLineCap = PenLineCap.Round
        };

        _boardCanvas.Children.Add(line1);
        _boardCanvas.Children.Add(line2);
    }

    private void DrawO(double centerX, double centerY, double size, bool isBotMark)
    {
        var color = new SolidColorBrush(isBotMark ? Color.Parse("#89b4fa") : Color.Parse("#f38ba8"));

        var ellipse = new Ellipse
        {
            Width = size,
            Height = size,
            Stroke = color,
            StrokeThickness = 3
        };
        Canvas.SetLeft(ellipse, centerX - size / 2);
        Canvas.SetTop(ellipse, centerY - size / 2);
        _boardCanvas.Children.Add(ellipse);
    }

    private char GetBotMark(TicTacToeGameState state)
    {
        if (state.PlayerX?.PlayerId == _botClientId)
            return 'X';
        return 'O';
    }

    private void DrawWinningLine(WinningLine winLine, double cellSize, double offsetX, double offsetY)
    {
        var startX = offsetX + winLine.StartX * cellSize + cellSize / 2;
        var startY = offsetY + winLine.StartY * cellSize + cellSize / 2;
        var endX = offsetX + winLine.EndX * cellSize + cellSize / 2;
        var endY = offsetY + winLine.EndY * cellSize + cellSize / 2;

        var line = new Line
        {
            StartPoint = new Point(startX, startY),
            EndPoint = new Point(endX, endY),
            Stroke = new SolidColorBrush(Color.Parse("#f9e2af")),
            StrokeThickness = 5,
            StrokeLineCap = PenLineCap.Round
        };
        _boardCanvas.Children.Add(line);
    }
}
