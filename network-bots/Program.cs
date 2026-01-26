using MiniGameNetworkBot;
using MiniGameNetworkBot.TicTacToe;
using MiniGameNetworkBot.TicTacToe.LocalGame;

var settings = Settings.Parse(args);

if (settings.Mode == RunMode.Train)
{
    RunTraining(settings);
    return;
}

await RunGame(settings);

static void RunTraining(Settings settings)
{
    Console.WriteLine("Starting curriculum training...");
    Console.WriteLine($"  Games per iteration: {settings.GamesPerIteration}");
    Console.WriteLine($"  Model path: {settings.ModelPath}");

    var network = new PolicyValueNetwork();
    var trainer = new SelfPlayTrainer(network);
    trainer.TrainWithCurriculum(
        gamesPerIteration: settings.GamesPerIteration,
        epochs: 5);

    network.SaveModel(settings.ModelPath);
    Console.WriteLine($"Training complete! Model saved to {settings.ModelPath}");
}

static async Task RunGame(Settings settings)
{
    var bot = BotFactory.Create(settings.BotType, settings.ModelPath);
    Console.WriteLine($"Using bot: {bot.Name}");

    using var cts = new CancellationTokenSource();
    Console.CancelKeyPress += (_, eventArgs) =>
    {
        eventArgs.Cancel = true;
        cts.Cancel();
    };

    await using var networkHandler = await GameNetworkHandler.ConnectAsync(
        settings.ServerAddress,
        bot.Name,
        CancellationToken.None);

    var opponentType = BotFactory.ToServerBotType(settings.OpponentType);

    var botTask = Task.Run(async () =>
    {
        try
        {
            var runner = new TicTacToeRunner(networkHandler, bot, opponentType);
            var won = await runner.RunAsync(cts.Token);
            Console.WriteLine(won ? "We won!" : "We lost!");
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Bot error: {ex.Message}");
        }
        finally
        {
            if (bot is IDisposable disposable)
                disposable.Dispose();
        }
    });

    if (settings.ShowUi)
    {
        TicTacToeUi.Run(networkHandler, cts.Token);
        await cts.CancelAsync();
        await botTask.WaitAsync(TimeSpan.FromSeconds(2));
    }
    else
    {
        await botTask.WaitAsync(cts.Token);
    }

    await networkHandler.DisposeAsync();
    Environment.Exit(0);
}
