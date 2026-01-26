namespace MiniGameNetworkBot.TicTacToe.LocalGame;

public sealed class NeuralMcts
{
    private readonly PolicyValueNetwork _network;
    private readonly int _simulations;
    private readonly float _explorationConstant;
    private readonly Random _random = new();

    public NeuralMcts(PolicyValueNetwork network, int simulations = 100, float explorationConstant = 1.4f)
    {
        _network = network;
        _simulations = simulations;
        _explorationConstant = explorationConstant;
    }

    public (int X, int Y) GetBestMove(GameEngine engine, float temperature = 0.1f)
    {
        var moves = engine.GetAvailableMoves();

        if (moves.Count == 0)
            return (engine.Width / 2, engine.Height / 2);

        if (moves.Count == 1)
            return moves[0];

        // Quick check for immediate win
        foreach (var (x, y) in moves)
        {
            if (IsWinningMove(engine, x, y, engine.CurrentPlayer))
                return (x, y);
        }

        // Quick check for immediate block
        var opponent = engine.CurrentPlayer == Mark.X ? Mark.O : Mark.X;
        foreach (var (x, y) in moves)
        {
            if (IsWinningMove(engine, x, y, opponent))
                return (x, y);
        }

        var root = new MctsNode(null, (-1, -1), engine.CurrentPlayer);

        // Initialize root with policy from network
        var state = engine.GetBoardState(engine.CurrentPlayer);
        var (policy, _) = _network.Predict(state);
        root.Expand(moves, policy, engine.Width);

        // Run simulations
        for (var i = 0; i < _simulations; i++)
        {
            var node = root;
            var simEngine = engine.Clone();

            // Selection - traverse to leaf
            while (node.IsExpanded && !simEngine.IsGameOver)
            {
                node = SelectChild(node);
                simEngine.Place(node.Move.X, node.Move.Y);
            }

            // Expansion and evaluation
            float value;
            if (simEngine.IsGameOver)
            {
                if (simEngine.Winner == null)
                    value = 0f;
                else if (simEngine.Winner == engine.CurrentPlayer)
                    value = 1f;
                else
                    value = -1f;
            }
            else
            {
                var leafState = simEngine.GetBoardState(simEngine.CurrentPlayer);
                var (leafPolicy, leafValue) = _network.Predict(leafState);
                var leafMoves = simEngine.GetAvailableMoves();

                if (leafMoves.Count > 0)
                    node.Expand(leafMoves, leafPolicy, simEngine.Width);

                // Value is from current player's perspective, need to flip if different
                value = simEngine.CurrentPlayer == engine.CurrentPlayer ? leafValue : -leafValue;
            }

            // Backpropagation
            while (node != null)
            {
                node.Visits++;
                node.TotalValue += node.Player == engine.CurrentPlayer ? value : -value;
                node = node.Parent;
            }
        }

        // Select move based on visit counts
        if (temperature < 0.01f)
        {
            // Greedy selection
            return root.Children.MaxBy(c => c.Visits)!.Move;
        }
        else
        {
            // Probabilistic selection based on visit counts
            var visitSum = root.Children.Sum(c => Math.Pow(c.Visits, 1.0 / temperature));
            var r = _random.NextDouble() * visitSum;
            var cumulative = 0.0;

            foreach (var child in root.Children)
            {
                cumulative += Math.Pow(child.Visits, 1.0 / temperature);
                if (r <= cumulative)
                    return child.Move;
            }

            return root.Children[^1].Move;
        }
    }

    public List<(float[] State, int Action, float Value)> GetTrainingData(GameEngine engine)
    {
        var root = new MctsNode(null, (-1, -1), engine.CurrentPlayer);
        var moves = engine.GetAvailableMoves();

        if (moves.Count == 0)
            return [];

        var state = engine.GetBoardState(engine.CurrentPlayer);
        var (policy, _) = _network.Predict(state);
        root.Expand(moves, policy, engine.Width);

        // Run simulations
        for (var i = 0; i < _simulations; i++)
        {
            var node = root;
            var simEngine = engine.Clone();

            while (node.IsExpanded && !simEngine.IsGameOver)
            {
                node = SelectChild(node);
                simEngine.Place(node.Move.X, node.Move.Y);
            }

            float value;
            if (simEngine.IsGameOver)
            {
                if (simEngine.Winner == null)
                    value = 0f;
                else if (simEngine.Winner == engine.CurrentPlayer)
                    value = 1f;
                else
                    value = -1f;
            }
            else
            {
                var leafState = simEngine.GetBoardState(simEngine.CurrentPlayer);
                var (leafPolicy, leafValue) = _network.Predict(leafState);
                var leafMoves = simEngine.GetAvailableMoves();

                if (leafMoves.Count > 0)
                    node.Expand(leafMoves, leafPolicy, simEngine.Width);

                value = simEngine.CurrentPlayer == engine.CurrentPlayer ? leafValue : -leafValue;
            }

            while (node != null)
            {
                node.Visits++;
                node.TotalValue += node.Player == engine.CurrentPlayer ? value : -value;
                node = node.Parent;
            }
        }

        // Create training target: improved policy from MCTS visit counts
        var improvedPolicy = new float[engine.Width * engine.Height];
        var totalVisits = root.Children.Sum(c => c.Visits);

        foreach (var child in root.Children)
        {
            var idx = child.Move.Y * engine.Width + child.Move.X;
            improvedPolicy[idx] = (float)child.Visits / totalVisits;
        }

        // Find best action (most visited)
        var bestChild = root.Children.MaxBy(c => c.Visits)!;
        var bestAction = bestChild.Move.Y * engine.Width + bestChild.Move.X;

        // Estimated value from root
        var rootValue = (float)(root.TotalValue / Math.Max(root.Visits, 1));

        return [(state, bestAction, rootValue)];
    }

    private MctsNode SelectChild(MctsNode node)
    {
        var logParentVisits = MathF.Log(node.Visits + 1);

        return node.Children.MaxBy(c =>
        {
            var q = c.Visits > 0 ? (float)(c.TotalValue / c.Visits) : 0f;
            var u = _explorationConstant * c.Prior * MathF.Sqrt(logParentVisits) / (1 + c.Visits);
            return q + u;
        })!;
    }

    private static bool IsWinningMove(GameEngine engine, int x, int y, Mark mark)
    {
        int[] dx = [1, 0, 1, 1];
        int[] dy = [0, 1, 1, -1];

        for (var d = 0; d < 4; d++)
        {
            var count = 1;

            for (var i = 1; i < engine.WinCount; i++)
            {
                var nx = x + dx[d] * i;
                var ny = y + dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                    break;
                if (engine.GetCell(nx, ny) != mark)
                    break;
                count++;
            }

            for (var i = 1; i < engine.WinCount; i++)
            {
                var nx = x - dx[d] * i;
                var ny = y - dy[d] * i;
                if (nx < 0 || ny < 0 || nx >= engine.Width || ny >= engine.Height)
                    break;
                if (engine.GetCell(nx, ny) != mark)
                    break;
                count++;
            }

            if (count >= engine.WinCount)
                return true;
        }

        return false;
    }

    private sealed class MctsNode
    {
        public MctsNode? Parent { get; }
        public (int X, int Y) Move { get; }
        public Mark Player { get; }
        public List<MctsNode> Children { get; } = [];
        public int Visits { get; set; }
        public double TotalValue { get; set; }
        public float Prior { get; set; }
        public bool IsExpanded => Children.Count > 0;

        public MctsNode(MctsNode? parent, (int X, int Y) move, Mark player)
        {
            Parent = parent;
            Move = move;
            Player = player;
        }

        public void Expand(List<(int X, int Y)> moves, float[] policy, int width)
        {
            var nextPlayer = Player == Mark.X ? Mark.O : Mark.X;

            foreach (var (x, y) in moves)
            {
                var prior = policy[y * width + x];
                var child = new MctsNode(this, (x, y), nextPlayer) { Prior = prior };
                Children.Add(child);
            }
        }
    }
}
