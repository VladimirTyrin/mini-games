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

            ShutdownToken.Register(() =>
            {
                Dispatcher.UIThread.InvokeAsync(() => desktop.MainWindow?.Close());
            });
        }

        base.OnFrameworkInitializationCompleted();
    }
}
