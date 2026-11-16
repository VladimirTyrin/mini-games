using Avalonia;

namespace MiniGameNetworkBot.TicTacToe;

public static class TicTacToeUi
{
    public static Action? OnWindowClosed { get; set; }

    /// <summary>Blocks until the window closes. Updates before the window opens are not shown; the next one redraws everything.</summary>
    public static void Run(TicTacToeRunner runner, string title, CancellationToken shutdownToken, Action onWindowClosed)
    {
        App.Runner = runner;
        App.WindowTitle = title;
        App.ShutdownToken = shutdownToken;
        OnWindowClosed = onWindowClosed;

        BuildAvaloniaApp().StartWithClassicDesktopLifetime([]);
    }

    private static AppBuilder BuildAvaloniaApp()
        => AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .LogToTrace();
}
