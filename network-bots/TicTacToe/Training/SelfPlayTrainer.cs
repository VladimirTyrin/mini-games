using MiniGameNetworkBot.TicTacToe.Bots;
using MiniGameNetworkBot.TicTacToe.Core;
using static TorchSharp.torch;

namespace MiniGameNetworkBot.TicTacToe.Training;

public enum TrainingOpponent
{
    Self,
    Random,
    Minimax,
    Mcts
}

public sealed class SelfPlayTrainer
{
    private readonly PolicyValueNetwork _network;
    private readonly optim.Optimizer _optimizer;
    private readonly Device _device;
    private readonly int _width;
    private readonly int _height;
    private readonly Random _random = new();

    private readonly Dictionary<TrainingOpponent, IBot> _opponents;
    private readonly List<(TrainingOpponent Opponent, float Weight)> _opponentMix;

    public SelfPlayTrainer(PolicyValueNetwork network, double learningRate = 0.001)
    {
        _network = network;
        _width = 15;
        _height = 15;

        _device = cuda.is_available() ? CUDA : CPU;
        Console.WriteLine($"[Training] Using device: {_device}");
        _network.MoveToDevice(_device);

        _optimizer = optim.Adam(_network.parameters(), lr: learningRate);

        _opponents = new Dictionary<TrainingOpponent, IBot>
        {
            [TrainingOpponent.Random] = new RandomBot(),
            [TrainingOpponent.Minimax] = new MinimaxBot(maxDepth: 1),
            [TrainingOpponent.Mcts] = new MctsBot(simulationsPerMove: 1000)
        };

        _opponentMix =
        [
            (TrainingOpponent.Self, 0.4f),
            (TrainingOpponent.Random, 0.3f),
            (TrainingOpponent.Minimax, 0.3f)
        ];
    }

    public void SetOpponentMix(List<(TrainingOpponent Opponent, float Weight)> mix)
    {
        _opponentMix.Clear();
        _opponentMix.AddRange(mix);
    }

    public void Train(int iterations = 100, int gamesPerIteration = 50, int epochs = 5)
    {
        Console.WriteLine("[Training] Opponent mix:");
        foreach (var (opponent, weight) in _opponentMix)
            Console.WriteLine($"  {opponent}: {weight * 100:F0}%");

        for (var iter = 0; iter < iterations; iter++)
        {
            Console.WriteLine($"\n=== Iteration {iter + 1}/{iterations} ===");

            var trainingData = CollectTrainingData(gamesPerIteration);
            Console.WriteLine($"Collected {trainingData.Count} positions from {gamesPerIteration} games");

            var (policyLoss, valueLoss) = TrainOnData(trainingData, epochs);
            Console.WriteLine($"Policy loss: {policyLoss:F4}, Value loss: {valueLoss:F4}");

            if ((iter + 1) % 10 == 0)
            {
                EvaluateProgress();
            }
        }
    }

    public void TrainWithCurriculum(int gamesPerIteration = 100, int epochs = 5)
    {
        Console.WriteLine("\n[Curriculum] Phase 1: Imitation Learning from Minimax");
        TrainImitation(iterations: 30, positionsPerIteration: 1000, epochs: 5, minimaxDepth: 1);

        Console.WriteLine("\n[Curriculum] Phase 2: MCTS Self-Play Refinement");
        TrainAlphaZero(iterations: 20, gamesPerIteration: 30, mctsSimulations: 50, epochs: 3);
    }

    public void TrainImitation(int iterations = 20, int positionsPerIteration = 500, int epochs = 5, int minimaxDepth = 2)
    {
        Console.WriteLine("[Training] Imitation Learning from Minimax");
        Console.WriteLine($"  Minimax depth: {minimaxDepth}");
        Console.WriteLine($"  Positions per iteration: {positionsPerIteration}");

        var teacher = new MinimaxBot(maxDepth: minimaxDepth);

        for (var iter = 0; iter < iterations; iter++)
        {
            Console.WriteLine($"\n=== Imitation Iteration {iter + 1}/{iterations} ===");

            var trainingData = CollectImitationData(positionsPerIteration, teacher);
            Console.WriteLine($"Collected {trainingData.Count} positions");

            var (policyLoss, valueLoss) = TrainOnMctsData(trainingData, epochs);
            Console.WriteLine($"Policy loss: {policyLoss:F4}, Value loss: {valueLoss:F4}");

            if ((iter + 1) % 5 == 0)
            {
                EvaluateImitationProgress(minimaxDepth);
            }
        }
    }

    private List<(float[] State, int Action, float Value)> CollectImitationData(int numPositions, IBot teacher)
    {
        var data = new List<(float[] State, int Action, float Value)>();

        while (data.Count < numPositions)
        {
            var engine = new GameEngine(_width, _height);

            var randomMoves = _random.Next(0, 20);
            for (var i = 0; i < randomMoves && !engine.IsGameOver; i++)
            {
                var moves = engine.GetAvailableMoves();
                if (moves.Count == 0) break;
                var (x, y) = moves[_random.Next(moves.Count)];
                engine.Place(x, y);
            }

            if (engine.IsGameOver)
                continue;

            var (teacherX, teacherY) = teacher.GetMove(engine);
            var state = engine.GetBoardState(engine.CurrentPlayer);
            var action = teacherY * _width + teacherX;

            var simEngine = engine.Clone();
            simEngine.Place(teacherX, teacherY);

            while (!simEngine.IsGameOver)
            {
                var moves = simEngine.GetAvailableMoves();
                if (moves.Count == 0) break;
                var (x, y) = teacher.GetMove(simEngine);
                simEngine.Place(x, y);
            }

            float value;
            if (simEngine.Winner == null)
                value = 0f;
            else if (simEngine.Winner == engine.CurrentPlayer)
                value = 1f;
            else
                value = -1f;

            data.Add((state, action, value));
        }

        return data;
    }

    private void EvaluateImitationProgress(int minimaxDepth)
    {
        _network.eval();

        var vsRandom = EvaluateAgainst(new RandomBot(), 20);
        Console.WriteLine($"Neural vs Random: {vsRandom * 100:F1}%");

        var vsMinimax1 = EvaluateAgainst(new MinimaxBot(maxDepth: 1), 20);
        Console.WriteLine($"Neural vs Minimax(d=1): {vsMinimax1 * 100:F1}%");

        var hybridBot = new HybridNeuralBot(_network);
        var hybridVsMinimax = EvaluateAgainst(hybridBot, new MinimaxBot(maxDepth: 1), 20);
        Console.WriteLine($"Hybrid vs Minimax(d=1): {hybridVsMinimax * 100:F1}%");

        if (minimaxDepth >= 2)
        {
            var vsMinimax2 = EvaluateAgainst(new MinimaxBot(maxDepth: 2), 10);
            Console.WriteLine($"Neural vs Minimax(d=2): {vsMinimax2 * 100:F1}%");
        }
    }

    public void TrainAlphaZero(int iterations = 50, int gamesPerIteration = 50, int mctsSimulations = 50, int epochs = 5)
    {
        Console.WriteLine("[Training] AlphaZero-style (MCTS + Neural)");
        Console.WriteLine($"  MCTS simulations per move: {mctsSimulations}");

        for (var iter = 0; iter < iterations; iter++)
        {
            Console.WriteLine($"\n=== Iteration {iter + 1}/{iterations} ===");

            var trainingData = CollectMctsData(gamesPerIteration, mctsSimulations);
            Console.WriteLine($"Collected {trainingData.Count} positions from {gamesPerIteration} games");

            var (policyLoss, valueLoss) = TrainOnMctsData(trainingData, epochs);
            Console.WriteLine($"Policy loss: {policyLoss:F4}, Value loss: {valueLoss:F4}");

            if ((iter + 1) % 5 == 0)
            {
                EvaluateMctsProgress(mctsSimulations);
            }
        }
    }

    private List<(float[] State, int Action, float Value)> CollectMctsData(int numGames, int simulations)
    {
        var data = new List<(float[] State, int Action, float Value)>();
        var mcts = new NeuralMcts(_network, simulations);

        for (var game = 0; game < numGames; game++)
        {
            var gameData = new List<(float[] State, int Action, Mark Player)>();
            var engine = new GameEngine(_width, _height);

            while (!engine.IsGameOver)
            {
                var moves = engine.GetAvailableMoves();
                if (moves.Count == 0) break;

                var mctsData = mcts.GetTrainingData(engine);
                if (mctsData.Count > 0)
                {
                    var (state, action, _) = mctsData[0];
                    gameData.Add((state, action, engine.CurrentPlayer));
                }

                var move = mcts.GetBestMove(engine, temperature: 1.0f);
                engine.Place(move.X, move.Y);
            }

            foreach (var (state, action, player) in gameData)
            {
                float reward;
                if (engine.Winner == null)
                    reward = 0f;
                else if (engine.Winner == player)
                    reward = 1f;
                else
                    reward = -1f;

                data.Add((state, action, reward));
            }
        }

        return data;
    }

    private (float PolicyLoss, float ValueLoss) TrainOnMctsData(
        List<(float[] State, int Action, float Value)> data,
        int epochs)
    {
        _network.train();

        var totalPolicyLoss = 0f;
        var totalValueLoss = 0f;
        var batches = 0;

        const int batchSize = 32;

        for (var epoch = 0; epoch < epochs; epoch++)
        {
            for (var i = data.Count - 1; i > 0; i--)
            {
                var j = _random.Next(i + 1);
                (data[i], data[j]) = (data[j], data[i]);
            }

            for (var i = 0; i < data.Count; i += batchSize)
            {
                var batchEnd = Math.Min(i + batchSize, data.Count);
                var batch = data.Skip(i).Take(batchEnd - i).ToList();

                var states = new float[batch.Count * 3 * _height * _width];
                var actions = new long[batch.Count];
                var values = new float[batch.Count];

                for (var j = 0; j < batch.Count; j++)
                {
                    Array.Copy(batch[j].State, 0, states, j * 3 * _height * _width, 3 * _height * _width);
                    actions[j] = batch[j].Action;
                    values[j] = batch[j].Value;
                }

                using var stateTensor = tensor(states, new long[] { batch.Count, 3, _height, _width }, device: _device);
                using var actionTensor = tensor(actions, device: _device);
                using var valueTensor = tensor(values, device: _device).view(-1, 1);

                var (policyOut, valueOut) = _network.forward(stateTensor);

                var policyLoss = nn.functional.nll_loss(policyOut, actionTensor);
                var valueLoss = nn.functional.mse_loss(valueOut, valueTensor);
                var loss = policyLoss + valueLoss;

                _optimizer.zero_grad();
                loss.backward();
                _optimizer.step();

                totalPolicyLoss += policyLoss.item<float>();
                totalValueLoss += valueLoss.item<float>();
                batches++;

                policyOut.Dispose();
                valueOut.Dispose();
                policyLoss.Dispose();
                valueLoss.Dispose();
                loss.Dispose();
            }
        }

        return (totalPolicyLoss / batches, totalValueLoss / batches);
    }

    private void EvaluateMctsProgress(int simulations)
    {
        _network.eval();

        var mcts = new NeuralMcts(_network, simulations);
        var mctsBot = new NeuralMctsBot(mcts);

        var vsRandom = EvaluateAgainst(mctsBot, new RandomBot(), 10);
        Console.WriteLine($"MCTS+Neural vs Random: {vsRandom * 100:F1}%");

        var vsMinimax = EvaluateAgainst(mctsBot, new MinimaxBot(maxDepth: 1), 10);
        Console.WriteLine($"MCTS+Neural vs Minimax(d=1): {vsMinimax * 100:F1}%");
    }

    private float EvaluateAgainst(IBot bot, IBot opponent, int numGames)
    {
        var wins = 0;

        for (var game = 0; game < numGames; game++)
        {
            var engine = new GameEngine(_width, _height);
            var botPlaysX = game % 2 == 0;
            var botMark = botPlaysX ? Mark.X : Mark.O;

            while (!engine.IsGameOver)
            {
                var moves = engine.GetAvailableMoves();
                if (moves.Count == 0) break;

                var isBotTurn = engine.CurrentPlayer == botMark;
                var (x, y) = isBotTurn ? bot.GetMove(engine) : opponent.GetMove(engine);
                engine.Place(x, y);
            }

            if (engine.Winner == botMark)
                wins++;
        }

        return (float)wins / numGames;
    }

    private sealed class NeuralMctsBot : IBot
    {
        private readonly NeuralMcts _mcts;
        public string Name => "MCTS+Neural";

        public NeuralMctsBot(NeuralMcts mcts) => _mcts = mcts;

        public (int X, int Y) GetMove(IBoardView board)
        {
            var engine = board as GameEngine ?? GameEngine.FromBoard(board);
            return _mcts.GetBestMove(engine, temperature: 0.1f);
        }
    }

    private void TrainPhase(int maxIterations, int gamesPerIteration, int epochs, float targetWinRate, TrainingOpponent opponent)
    {
        Console.WriteLine($"[Phase] Target: {targetWinRate * 100:F0}% win rate vs {opponent}");

        for (var iter = 0; iter < maxIterations; iter++)
        {
            Console.WriteLine($"\n=== Phase Iteration {iter + 1}/{maxIterations} ===");

            var trainingData = CollectTrainingData(gamesPerIteration);
            Console.WriteLine($"Collected {trainingData.Count} positions");

            var (policyLoss, valueLoss) = TrainOnData(trainingData, epochs);
            Console.WriteLine($"Policy loss: {policyLoss:F4}, Value loss: {valueLoss:F4}");

            if ((iter + 1) % 5 == 0)
            {
                var winRate = EvaluateAgainst(_opponents[opponent], 20);
                Console.WriteLine($"Win rate vs {opponent}: {winRate * 100:F1}%");

                if (winRate >= targetWinRate)
                {
                    Console.WriteLine($"[Phase] Target reached! Moving to next phase.");
                    break;
                }
            }
        }
    }

    private List<(float[] State, int Action, float Reward, Mark Player)> CollectTrainingData(int numGames)
    {
        var data = new List<(float[] State, int Action, float Reward, Mark Player)>();
        var gamesPerOpponent = DistributeGames(numGames);

        foreach (var (opponent, games) in gamesPerOpponent)
        {
            for (var i = 0; i < games; i++)
            {
                var gameData = opponent == TrainingOpponent.Self
                    ? PlaySelfPlayGame()
                    : PlayAgainstOpponent(_opponents[opponent]);

                data.AddRange(gameData);
            }
        }

        return data;
    }

    private Dictionary<TrainingOpponent, int> DistributeGames(int totalGames)
    {
        var result = new Dictionary<TrainingOpponent, int>();
        var remaining = totalGames;

        for (var i = 0; i < _opponentMix.Count - 1; i++)
        {
            var (opponent, weight) = _opponentMix[i];
            var games = (int)(totalGames * weight);
            result[opponent] = games;
            remaining -= games;
        }

        if (_opponentMix.Count > 0)
            result[_opponentMix[^1].Opponent] = remaining;

        return result;
    }

    private List<(float[] State, int Action, float Reward, Mark Player)> PlaySelfPlayGame()
    {
        var data = new List<(float[] State, int Action, float Reward, Mark Player)>();
        var gameHistory = new List<(float[] State, int Action, Mark Player)>();
        var engine = new GameEngine(_width, _height);

        while (!engine.IsGameOver)
        {
            var state = engine.GetBoardState(engine.CurrentPlayer);
            var availableMoves = engine.GetAvailableMoves();

            if (availableMoves.Count == 0)
                break;

            var move = SelectMove(state, availableMoves, temperature: 1.0f);
            gameHistory.Add((state, move.Y * _width + move.X, engine.CurrentPlayer));
            engine.Place(move.X, move.Y);
        }

        foreach (var (state, action, player) in gameHistory)
        {
            float reward;
            if (engine.Winner == null)
                reward = 0f;
            else if (engine.Winner == player)
                reward = 1f;
            else
                reward = -1f;

            data.Add((state, action, reward, player));
        }

        return data;
    }

    private List<(float[] State, int Action, float Reward, Mark Player)> PlayAgainstOpponent(IBot opponent)
    {
        var data = new List<(float[] State, int Action, float Reward, Mark Player)>();
        var gameHistory = new List<(float[] State, int Action, Mark Player)>();
        var engine = new GameEngine(_width, _height);

        var networkPlaysX = _random.Next(2) == 0;
        var networkMark = networkPlaysX ? Mark.X : Mark.O;

        while (!engine.IsGameOver)
        {
            var availableMoves = engine.GetAvailableMoves();
            if (availableMoves.Count == 0)
                break;

            var isNetworkTurn = engine.CurrentPlayer == networkMark;

            if (isNetworkTurn)
            {
                var state = engine.GetBoardState(engine.CurrentPlayer);
                var move = SelectMove(state, availableMoves, temperature: 1.0f);
                gameHistory.Add((state, move.Y * _width + move.X, engine.CurrentPlayer));
                engine.Place(move.X, move.Y);
            }
            else
            {
                var (x, y) = opponent.GetMove(engine);
                engine.Place(x, y);
            }
        }

        foreach (var (state, action, player) in gameHistory)
        {
            float reward;
            if (engine.Winner == null)
                reward = 0f;
            else if (engine.Winner == player)
                reward = 1f;
            else
                reward = -1f;

            data.Add((state, action, reward, player));
        }

        return data;
    }

    private (int X, int Y) SelectMove(float[] state, List<(int X, int Y)> availableMoves, float temperature)
    {
        var (policy, _) = _network.Predict(state);

        var moveProbabilities = new List<(int X, int Y, float Prob)>();
        var totalProb = 0f;

        foreach (var (x, y) in availableMoves)
        {
            var prob = policy[y * _width + x];
            if (temperature > 0)
                prob = MathF.Pow(prob, 1f / temperature);
            moveProbabilities.Add((x, y, prob));
            totalProb += prob;
        }

        if (totalProb <= 0)
            return availableMoves[_random.Next(availableMoves.Count)];

        var r = (float)_random.NextDouble() * totalProb;
        var cumulative = 0f;

        foreach (var (x, y, prob) in moveProbabilities)
        {
            cumulative += prob;
            if (r <= cumulative)
                return (x, y);
        }

        return availableMoves[^1];
    }

    private (float PolicyLoss, float ValueLoss) TrainOnData(
        List<(float[] State, int Action, float Reward, Mark Player)> data,
        int epochs)
    {
        _network.train();

        var totalPolicyLoss = 0f;
        var totalValueLoss = 0f;
        var batches = 0;

        const int batchSize = 32;

        for (var epoch = 0; epoch < epochs; epoch++)
        {
            Shuffle(data);

            for (var i = 0; i < data.Count; i += batchSize)
            {
                var batchEnd = Math.Min(i + batchSize, data.Count);
                var batch = data.Skip(i).Take(batchEnd - i).ToList();

                var states = new float[batch.Count * 3 * _height * _width];
                var actions = new long[batch.Count];
                var rewards = new float[batch.Count];

                for (var j = 0; j < batch.Count; j++)
                {
                    Array.Copy(batch[j].State, 0, states, j * 3 * _height * _width, 3 * _height * _width);
                    actions[j] = batch[j].Action;
                    rewards[j] = batch[j].Reward;
                }

                using var stateTensor = tensor(states, new long[] { batch.Count, 3, _height, _width }, device: _device);
                using var actionTensor = tensor(actions, device: _device);
                using var rewardTensor = tensor(rewards, device: _device).view(-1, 1);

                var (policyOut, valueOut) = _network.forward(stateTensor);

                var policyLoss = nn.functional.nll_loss(policyOut, actionTensor);
                var valueLoss = nn.functional.mse_loss(valueOut, rewardTensor);
                var loss = policyLoss + valueLoss;

                _optimizer.zero_grad();
                loss.backward();
                _optimizer.step();

                totalPolicyLoss += policyLoss.item<float>();
                totalValueLoss += valueLoss.item<float>();
                batches++;

                policyOut.Dispose();
                valueOut.Dispose();
                policyLoss.Dispose();
                valueLoss.Dispose();
                loss.Dispose();
            }
        }

        return (totalPolicyLoss / batches, totalValueLoss / batches);
    }

    private void EvaluateProgress()
    {
        _network.eval();

        var vsRandom = EvaluateAgainst(new RandomBot(), 20);
        Console.WriteLine($"Win rate vs Random: {vsRandom * 100:F1}%");

        var vsMinimax = EvaluateAgainst(_opponents[TrainingOpponent.Minimax], 10);
        Console.WriteLine($"Win rate vs Minimax: {vsMinimax * 100:F1}%");
    }

    private float EvaluateAgainst(IBot opponent, int numGames)
    {
        var wins = 0;

        for (var game = 0; game < numGames; game++)
        {
            var engine = new GameEngine(_width, _height);
            var networkPlaysX = game % 2 == 0;
            var networkMark = networkPlaysX ? Mark.X : Mark.O;

            while (!engine.IsGameOver)
            {
                var availableMoves = engine.GetAvailableMoves();
                if (availableMoves.Count == 0)
                    break;

                var isNetworkTurn = engine.CurrentPlayer == networkMark;

                if (isNetworkTurn)
                {
                    var state = engine.GetBoardState(engine.CurrentPlayer);
                    var move = SelectMove(state, availableMoves, temperature: 0.1f);
                    engine.Place(move.X, move.Y);
                }
                else
                {
                    var (x, y) = opponent.GetMove(engine);
                    engine.Place(x, y);
                }
            }

            if (engine.Winner == networkMark)
                wins++;
        }

        return (float)wins / numGames;
    }

    private void Shuffle<T>(List<T> list)
    {
        for (var i = list.Count - 1; i > 0; i--)
        {
            var j = _random.Next(i + 1);
            (list[i], list[j]) = (list[j], list[i]);
        }
    }
}
