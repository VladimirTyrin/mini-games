using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Threading;

namespace MiniGameNetworkBot.TicTacToe;

public class App : Application
{
    public static GameNetworkHandler? NetworkHandler { get; set; }
    public static CancellationToken ShutdownToken { get; set; }

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop && NetworkHandler != null)
        {
            desktop.ShutdownMode = ShutdownMode.OnMainWindowClose;
            desktop.MainWindow = new GameWindow(NetworkHandler);

            var isShuttingDown = false;

            desktop.ShutdownRequested += (_, _) =>
            {
                if (isShuttingDown) return;
                isShuttingDown = true;
                TicTacToeUi.OnWindowClosed?.Invoke();
            };

            ShutdownToken.Register(() =>
            {
                if (isShuttingDown) return;
                isShuttingDown = true;
                Dispatcher.UIThread.InvokeAsync(() => desktop.Shutdown());
            });
        }

        base.OnFrameworkInitializationCompleted();
    }
}
