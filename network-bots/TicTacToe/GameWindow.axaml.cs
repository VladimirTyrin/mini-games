using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Media;
using Avalonia.Threading;
using Minigames.V2;
using MiniGameNetworkBot.TicTacToe.Core;

namespace MiniGameNetworkBot.TicTacToe;

/// <summary>Shows the match the runner plays, as the core's view renders it.</summary>
public partial class GameWindow : Window
{
    private TicTacToeView? _view;
    private uint? _mySlot;

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

    public GameWindow(TicTacToeRunner runner, string title)
    {
        InitializeComponent();

        Title = title;
        _boardCanvas.SizeChanged += (_, _) => RedrawBoard();
        runner.ViewChanged += (view, mySlot) => Dispatcher.UIThread.Post(() => UpdateView(view, mySlot));
        runner.MatchEnded += (view, ended) => Dispatcher.UIThread.Post(() => ShowEnd(view, ended));
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

    private void UpdateView(TicTacToeView view, uint? mySlot)
    {
        _view = view;
        _mySlot = mySlot;

        var botMark = BotMark();
        _botMarkText.Text = botMark == Mark.O ? "O" : "X";
        _opponentMarkText.Text = botMark == Mark.O ? "X" : "O";
        _moveCountText.Text = $"Move: {view.Cells.Count(cell => cell != TicTacToeMark.Unspecified)}";
        _winConditionText.Text = $"Win: {view.WinCount} in a row";
        _boardSizeText.Text = $"Board: {view.FieldWidth}x{view.FieldHeight}";

        if (view.End is null)
        {
            var isOurTurn = view.HasCurrentSlot && view.CurrentSlot == mySlot;
            SetStatus(isOurTurn ? "Our turn - thinking..." : "Opponent's turn", isOurTurn);
        }

        RedrawBoard();
    }

    private void ShowEnd(TicTacToeView view, MatchEnded ended)
    {
        UpdateView(view, _mySlot);
        var won = ended.Result is { } result && _mySlot is { } slot && result.WinnerSlots.Contains(slot);
        var draw = ended.Result?.Kind == OutcomeKind.Draw;
        _statusText.Text = won ? "WE WON!" : draw ? "Draw" : "We lost";
        _statusText.Foreground = new SolidColorBrush(Color.Parse(won ? "#a6e3a1" : draw ? "#f9e2af" : "#f38ba8"));
        _statusText.FontSize = 24;
    }

    private Mark BotMark() =>
        _view is not null && _mySlot is { } slot ? TicTacToeViews.MarkOf(_view, slot) : Mark.Empty;

    private void SetStatus(string status, bool isOurTurn)
    {
        _statusText.Text = status;
        _statusText.Foreground = new SolidColorBrush(Color.Parse(isOurTurn ? "#a6e3a1" : "#f9e2af"));
    }

    private void RedrawBoard()
    {
        if (_view is null)
            return;

        var view = _view;
        _boardCanvas.Children.Clear();

        var width = (int)view.FieldWidth;
        var height = (int)view.FieldHeight;
        var canvasWidth = _boardCanvas.Bounds.Width;
        var canvasHeight = _boardCanvas.Bounds.Height;
        if (canvasWidth <= 0 || canvasHeight <= 0)
            return;

        var cellSize = Math.Min(canvasWidth / width, canvasHeight / height);
        var offsetX = (canvasWidth - cellSize * width) / 2;
        var offsetY = (canvasHeight - cellSize * height) / 2;

        DrawGrid(width, height, cellSize, offsetX, offsetY);
        DrawMarks(view, cellSize, offsetX, offsetY);
        if (view.End?.WinningLine is { } line)
            DrawWinningLine(line, cellSize, offsetX, offsetY);
    }

    private void DrawGrid(int width, int height, double cellSize, double offsetX, double offsetY)
    {
        var gridColor = new SolidColorBrush(Color.Parse("#45475a"));

        for (var i = 0; i <= width; i++)
        {
            _boardCanvas.Children.Add(new Line
            {
                StartPoint = new Point(offsetX + i * cellSize, offsetY),
                EndPoint = new Point(offsetX + i * cellSize, offsetY + height * cellSize),
                Stroke = gridColor,
                StrokeThickness = 1
            });
        }

        for (var i = 0; i <= height; i++)
        {
            _boardCanvas.Children.Add(new Line
            {
                StartPoint = new Point(offsetX, offsetY + i * cellSize),
                EndPoint = new Point(offsetX + width * cellSize, offsetY + i * cellSize),
                Stroke = gridColor,
                StrokeThickness = 1
            });
        }
    }

    private void DrawMarks(TicTacToeView view, double cellSize, double offsetX, double offsetY)
    {
        var botMark = BotMark();
        var width = (int)view.FieldWidth;

        for (var index = 0; index < view.Cells.Count; index++)
        {
            var mark = TicTacToeViews.ToMark(view.Cells[index]);
            if (mark == Mark.Empty)
                continue;

            var x = index % width;
            var y = index / width;
            var centerX = offsetX + x * cellSize + cellSize / 2;
            var centerY = offsetY + y * cellSize + cellSize / 2;
            var markSize = cellSize * 0.6;

            if (view.LastMove is { } last && last.X == x && last.Y == y)
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

            if (mark == Mark.X)
                DrawX(centerX, centerY, markSize, botMark == Mark.X);
            else
                DrawO(centerX, centerY, markSize, botMark == Mark.O);
        }
    }

    private void DrawX(double centerX, double centerY, double size, bool isBotMark)
    {
        var color = new SolidColorBrush(Color.Parse(isBotMark ? "#89b4fa" : "#f38ba8"));
        _boardCanvas.Children.Add(new Line
        {
            StartPoint = new Point(centerX - size / 2, centerY - size / 2),
            EndPoint = new Point(centerX + size / 2, centerY + size / 2),
            Stroke = color,
            StrokeThickness = 3,
            StrokeLineCap = PenLineCap.Round
        });
        _boardCanvas.Children.Add(new Line
        {
            StartPoint = new Point(centerX + size / 2, centerY - size / 2),
            EndPoint = new Point(centerX - size / 2, centerY + size / 2),
            Stroke = color,
            StrokeThickness = 3,
            StrokeLineCap = PenLineCap.Round
        });
    }

    private void DrawO(double centerX, double centerY, double size, bool isBotMark)
    {
        var ellipse = new Ellipse
        {
            Width = size,
            Height = size,
            Stroke = new SolidColorBrush(Color.Parse(isBotMark ? "#89b4fa" : "#f38ba8")),
            StrokeThickness = 3
        };
        Canvas.SetLeft(ellipse, centerX - size / 2);
        Canvas.SetTop(ellipse, centerY - size / 2);
        _boardCanvas.Children.Add(ellipse);
    }

    private void DrawWinningLine(TicTacToeWinningLine line, double cellSize, double offsetX, double offsetY)
    {
        if (line.Start is not { } start || line.End is not { } end)
            return;
        _boardCanvas.Children.Add(new Line
        {
            StartPoint = new Point(offsetX + start.X * cellSize + cellSize / 2, offsetY + start.Y * cellSize + cellSize / 2),
            EndPoint = new Point(offsetX + end.X * cellSize + cellSize / 2, offsetY + end.Y * cellSize + cellSize / 2),
            Stroke = new SolidColorBrush(Color.Parse("#f9e2af")),
            StrokeThickness = 5,
            StrokeLineCap = PenLineCap.Round
        });
    }
}
