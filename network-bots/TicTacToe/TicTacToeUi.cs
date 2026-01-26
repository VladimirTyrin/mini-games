using Avalonia;

namespace MiniGameNetworkBot.TicTacToe;

public static class TicTacToeUi
{
    public static void Run(GameNetworkHandler handler, CancellationToken shutdownToken)
    {
        App.NetworkHandler = handler;
        App.ShutdownToken = shutdownToken;

        BuildAvaloniaApp().StartWithClassicDesktopLifetime([]);
    }

    private static AppBuilder BuildAvaloniaApp()
        => AppBuilder.Configure<App>()
            .UsePlatformDetect()
            .LogToTrace();
}
