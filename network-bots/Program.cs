using MiniGameNetworkBot;
using MiniGameNetworkBot.Network;
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
        return await RunGame(settings);
}

return 0;

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
        bot2Name,
        settings.BoardSize,
        settings.BoardSize,
        settings.WinCount);

    var result = tournament.Run(
        settings.BenchmarkGames,
        settings.BenchmarkThreads,
        cts.Token);

    LocalTournament.PrintResult(result, bot1Name, bot2Name);
}

static async Task<int> RunGame(Settings settings)
{
    if (string.IsNullOrEmpty(settings.BotKey))
    {
        Console.Error.WriteLine("A bot key is required: --bot-key KEY or the MG_BOT_KEY environment variable.");
        return 2;
    }

    var bot = BotFactory.Create(settings.BotType, settings.ModelPath, settings.MinimaxDepth);
    Console.WriteLine($"Using bot: {bot.Name}");

    using var cts = new CancellationTokenSource();
    Console.CancelKeyPress += (_, eventArgs) =>
    {
        eventArgs.Cancel = true;
        cts.Cancel();
    };

    await using var session = await BotSession.ConnectAsync(new Uri(settings.ServerUrl), settings.BotKey, cts.Token)
        .ConfigureAwait(false);
    Console.WriteLine($"Signed in as {session.Identity.DisplayName} at {settings.ServerUrl}");

    var runner = new TicTacToeRunner(session, bot, BotFactory.ToServerBotKind(settings.OpponentType));
    var exitCode = 1;
    var botTask = Task.Run(async () =>
    {
        try
        {
            var ended = await runner.RunAsync(cts.Token).ConfigureAwait(false);
            var won = ended.Result is { } result && result.WinnerSlots.Count == 1
                                                 && runner.MySlot is { } slot && result.WinnerSlots.Contains(slot);
            Console.WriteLine(ended.Result is null ? "The server aborted the match."
                : won ? "We won!"
                : ended.Result.Kind == Minigames.V2.OutcomeKind.Draw ? "Draw."
                : "We lost!");
            exitCode = 0;
        }
        catch (OperationCanceledException)
        {
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Bot error: {ex.Message}");
        }
    });

    if (settings.ShowUi)
    {
        TicTacToeUi.Run(runner, $"{bot.Name} [{session.Identity.DisplayName}]", cts.Token, () => cts.Cancel());
        await botTask.WaitAsync(TimeSpan.FromSeconds(2)).ConfigureAwait(ConfigureAwaitOptions.SuppressThrowing);
    }
    else
    {
        await botTask.WaitAsync(cts.Token).ConfigureAwait(ConfigureAwaitOptions.SuppressThrowing);
    }

    return exitCode;
}
