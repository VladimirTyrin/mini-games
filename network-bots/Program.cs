using MiniGameNetworkBot;
using MiniGameNetworkBot.TicTacToe;

var serverAddress = args.Contains("--local", StringComparer.OrdinalIgnoreCase)
    ? "http://localhost:5001"
    : "https://braintvsminigames.xyz:5443";

var showUi = !args.Contains("--no-ui", StringComparer.OrdinalIgnoreCase);

using var cts = new CancellationTokenSource();
Console.CancelKeyPress += (_, eventArgs) =>
{
    eventArgs.Cancel = true;
    cts.Cancel();
};

await using var networkHandler = await GameNetworkHandler.ConnectAsync(serverAddress, CancellationToken.None);

var botTask = Task.Run(async () =>
{
    try
    {
        var bot = new TicTacToeMinimaxBot();
        // ReSharper disable AccessToDisposedClosure
        var runner = new TicTacToeRunner(networkHandler, bot);
        var won = await runner.RunAsync(cts.Token);
        // ReSharper restore AccessToDisposedClosure
        Console.WriteLine(won ? "We won!" : "We lost!");
    }
    catch (OperationCanceledException)
    {
    }
    catch (Exception ex)
    {
        Console.WriteLine($"Bot error: {ex.Message}");
    }
});

if (showUi)
{
    TicTacToeUi.Run(networkHandler, cts.Token);
    await cts.CancelAsync();
    await botTask.WaitAsync(TimeSpan.FromSeconds(2));
    await networkHandler.DisposeAsync();
    Environment.Exit(0);
}
else
{
    await botTask.WaitAsync(cts.Token);
}


