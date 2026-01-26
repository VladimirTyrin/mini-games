using MiniGameNetworkBot;
using MiniGameNetworkBot.TicTacToe;

var serverAddress = args.Contains("--local", StringComparer.OrdinalIgnoreCase)
    ? "http://localhost:5001"
    : "https://braintvsminigames.xyz:5443";

using var cts = new CancellationTokenSource();
Console.CancelKeyPress += (_, eventArgs) => 
{
    eventArgs.Cancel = true;
    // ReSharper disable once AccessToDisposedClosure
    cts.Cancel();
};

await using var networkHandler = await GameNetworkHandler.ConnectAsync(serverAddress, CancellationToken.None);
var bot = new TicTacToeMinimaxBot();
var runner = new TicTacToeRunner(networkHandler, bot);
var won = await runner.RunAsync(cts.Token);
Console.WriteLine(won ? "Our bot won!" : "Our bot lost!");