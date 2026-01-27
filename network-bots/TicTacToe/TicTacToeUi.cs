using Avalonia;

namespace MiniGameNetworkBot.TicTacToe;

public static class TicTacToeUi
{
    public static Action? OnWindowClosed { get; set; }

    public static void Run(GameNetworkHandler handler, CancellationToken shutdownToken, Action onWindowClosed)
    {
        App.NetworkHandler = handler;
        App.ShutdownToken = shutdownToken;
        OnWindowClosed = onWindowClosed;

        BuildAvaloniaApp().StartWithClassicDesktopLifetime([]);
    }

    private static AppBuilder BuildAvaloniaApp()
        => AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .LogToTrace();
}
