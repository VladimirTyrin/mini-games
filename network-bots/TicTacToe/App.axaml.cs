using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Threading;

namespace MiniGameNetworkBot.TicTacToe;

public class App : Application
{
    public static TicTacToeRunner? Runner { get; set; }
    public static string WindowTitle { get; set; } = "TicTacToe Bot";
    public static CancellationToken ShutdownToken { get; set; }

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop && Runner != null)
        {
            desktop.ShutdownMode = ShutdownMode.OnMainWindowClose;
            desktop.MainWindow = new GameWindow(Runner, WindowTitle);

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
