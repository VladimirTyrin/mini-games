using MiniGameNetworkBot;
using MiniGameNetworkBot.TicTacToe;
using MiniGameNetworkBot.TicTacToe.Adapters;
using MiniGameNetworkBot.TicTacToe.Training;

var settings = Settings.Parse(args);

switch (settings.Mode)
{
    case RunMode.Train:
        RunTraining(settings);
        break;
    case RunMode.Benchmark:
        RunBenchmark(settings);
        break;
    default:
        await RunGame(settings);
        break;
}

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

static void RunBenchmark(Settings settings)
{
    using var cts = new CancellationTokenSource();
    Console.CancelKeyPress += (_, eventArgs) =>
    {
        eventArgs.Cancel = true;
        cts.Cancel();
    };

    var bot1Name = BotFactory.GetBotName(settings.Bot1Type, settings.MinimaxDepth);
    var bot2Name = BotFactory.GetBotName(settings.Bot2Type, settings.MinimaxDepth);

    var bot1Factory = BotFactory.CreateFactory(settings.Bot1Type, settings.ModelPath, settings.MinimaxDepth);
    var bot2Factory = BotFactory.CreateFactory(settings.Bot2Type, settings.ModelPath, settings.MinimaxDepth);

    var tournament = new LocalTournament(
        bot1Factory,
        bot2Factory,
        bot1Name,
        bot2Name);

    var result = tournament.Run(
        settings.BenchmarkGames,
        settings.BenchmarkThreads,
        cts.Token);

    LocalTournament.PrintResult(result, bot1Name, bot2Name);
}

static async Task RunGame(Settings settings)
{
    var bot = BotFactory.Create(settings.BotType, settings.ModelPath, settings.MinimaxDepth);
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
