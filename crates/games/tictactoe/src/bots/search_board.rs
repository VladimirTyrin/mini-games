use super::candidate::Candidate;
use super::neighbourhood::Neighbourhood;
use super::probe::Probe;
use super::run::Run;
use super::side::Side;
use super::stone::Stone;
use super::window_counts::WindowCounts;
use crate::dimensions::Dimensions;
use crate::line_direction::LineDirection;
use crate::mark::Mark;
use crate::point::Point;
use crate::settings::MAX_FIELD_SIZE;
use crate::tictactoe::TicTacToe;
use crate::win_length::WinLength;

/// Longest line segment a probe looks at: `2 * win_count - 1` cells.
const MAX_SEGMENT: usize = 2 * MAX_FIELD_SIZE as usize - 1;
/// Window weight grows 8× per stone; capped so the sum over every window of a 20×20 board stays
/// far below `Score::HEURISTIC_LIMIT` (legacy `1 << (count * 2)` overflowed i32, T5).
const MAX_WEIGHT_SHIFT: usize = 39;
/// Cells `play`/`undo` update: the 5×5 square around the move.
const NEIGHBOURHOOD_WORK: u64 = 25;
const DIRECTIONS: u64 = LineDirection::ALL.len() as u64;

/// Mutable copy of the position for the bots, with a bot-perspective evaluation kept up to date
/// incrementally: the sum over every `win_count`-long window of `weight(bot stones)` if the
/// window has no opponent stones, `-weight(opponent stones)` if it has no bot stones, else 0.
#[derive(Clone, Debug)]
pub(crate) struct SearchBoard {
    dimensions: Dimensions,
    win_length: WinLength,
    stones: Vec<Stone>,
    /// Stones at Chebyshev distance 1 and up to 2 of each cell, kept incrementally: candidate
    /// generation is a single row-major scan instead of a set built per node.
    adjacent_stones: Vec<u16>,
    stones_within_two: Vec<u16>,
    stone_count: usize,
    evaluation: i64,
    weights: Vec<i64>,
    /// Cells visited so far by probes, candidate scans and `play`/`undo`: the search charges its
    /// budget with this, so the budget bounds real work whatever the board size and win length.
    work_done: u64,
    probe_work: u64,
    scan_work: u64,
}

impl SearchBoard {
    pub(crate) fn new(game: &TicTacToe, bot_mark: Mark) -> Self {
        let board = game.board();
        let dimensions = board.dimensions();
        let win_length = game.win_length();
        let cells = dimensions.cell_count();
        let weights = (0..=win_length.cells())
            .map(|stones| match stones {
                0 => 0,
                _ => 1_i64 << (3 * stones).min(MAX_WEIGHT_SHIFT),
            })
            .collect();
        let mut search_board = Self {
            dimensions,
            win_length,
            stones: vec![Stone::Empty; cells],
            adjacent_stones: vec![0; cells],
            stones_within_two: vec![0; cells],
            stone_count: 0,
            evaluation: 0,
            weights,
            work_done: 0,
            probe_work: DIRECTIONS * (2 * u64::from(win_length.get()) - 1),
            scan_work: u64::from(dimensions.width()) * u64::from(dimensions.height()),
        };
        for (index, (point, cell)) in dimensions.points().zip(board.cells()).enumerate() {
            let side = match *cell {
                None => continue,
                Some(mark) if mark == bot_mark => Side::Bot,
                Some(_) => Side::Opponent,
            };
            let candidate = Candidate {
                index,
                point,
                probe: search_board.probe(point),
            };
            search_board.play(&candidate, side);
        }
        search_board
    }

    pub(crate) fn evaluation(&self) -> i64 {
        self.evaluation
    }

    pub(crate) fn empty_cells(&self) -> usize {
        self.stones.len() - self.stone_count
    }

    pub(crate) fn work_done(&self) -> u64 {
        self.work_done
    }

    fn stone_along(&self, point: Point, direction: LineDirection, steps: i32) -> Stone {
        direction
            .walk(point, steps)
            .and_then(|cell| self.dimensions.index_of(cell))
            .and_then(|index| self.stones.get(index).copied())
            .unwrap_or(Stone::OffBoard)
    }

    /// Empty cells in `neighbourhood` of any stone, row-major, probes left at default. Widens to
    /// two cells, then to every empty cell, when nothing qualifies; the centre on an empty board.
    pub(crate) fn collect_candidates(
        &mut self,
        neighbourhood: Neighbourhood,
        candidates: &mut Vec<Candidate>,
    ) {
        candidates.clear();
        if neighbourhood == Neighbourhood::Adjacent {
            self.collect_where(candidates, |board, index| {
                board
                    .adjacent_stones
                    .get(index)
                    .is_some_and(|&near| near > 0)
            });
        }
        if candidates.is_empty() {
            self.collect_where(candidates, |board, index| {
                board
                    .stones_within_two
                    .get(index)
                    .is_some_and(|&near| near > 0)
            });
        }
        if !candidates.is_empty() {
            return;
        }
        if self.stone_count == 0 {
            let centre = self.dimensions.centre();
            if let Some(index) = self.dimensions.index_of(centre) {
                candidates.push(Candidate {
                    index,
                    point: centre,
                    probe: Probe::default(),
                });
            }
            return;
        }
        self.collect_where(candidates, |_, _| true);
    }

    /// Appends the empty cells for which `qualifies` holds, in row-major order.
    fn collect_where(
        &mut self,
        candidates: &mut Vec<Candidate>,
        qualifies: impl Fn(&Self, usize) -> bool,
    ) {
        self.work_done += self.scan_work;
        let empty_cells = self
            .dimensions
            .points()
            .zip(&self.stones)
            .enumerate()
            .filter(|(index, (_, stone))| **stone == Stone::Empty && qualifies(self, *index));
        candidates.extend(empty_cells.map(|(index, (point, _))| Candidate {
            index,
            point,
            probe: Probe::default(),
        }));
    }

    /// Scores placing either side's stone on the empty cell `point`: every window through it is
    /// visited once per direction by sliding over a `2 * win_count - 1` segment.
    pub(crate) fn probe(&mut self, point: Point) -> Probe {
        self.work_done += self.probe_work;
        let span = self.win_length.cells();
        let mut probe = Probe::default();
        let mut line = [Stone::OffBoard; MAX_SEGMENT];
        for direction in LineDirection::ALL {
            let Some(segment) = line.get_mut(..self.win_length.segment_cells()) else {
                continue;
            };
            self.read_segment(point, direction, segment);
            let segment = &*segment;
            let mut counts = WindowCounts::default();
            segment
                .iter()
                .take(span)
                .for_each(|&stone| counts.add(stone));
            self.score_window(counts, &mut probe);
            for (&leaving, &entering) in segment.iter().zip(segment.iter().skip(span)) {
                counts.remove(leaving);
                counts.add(entering);
                self.score_window(counts, &mut probe);
            }
        }
        probe
    }

    /// The cells `-reach..=reach` steps from the on-board `point` along `direction`. Probes
    /// dominate the search, so this uses index arithmetic instead of a walk per cell.
    fn read_segment(&self, point: Point, direction: LineDirection, segment: &mut [Stone]) {
        segment.fill(Stone::OffBoard);
        let Some(span) = self.dimensions.line_span(point, direction) else {
            return;
        };
        let reach = self.win_length.cells() - 1;
        let backward = reach.min(span.backward_steps);
        let forward = reach.min(span.forward_steps);
        let first_index = span.origin_index - backward * span.stride;
        let on_board = segment
            .iter_mut()
            .skip(reach - backward)
            .take(backward + 1 + forward);
        let indices = (first_index..).step_by(span.stride);
        for (stone, index) in on_board.zip(indices) {
            *stone = self.stones.get(index).copied().unwrap_or(Stone::OffBoard);
        }
    }

    fn weight(&self, stones: usize) -> i64 {
        self.weights.get(stones).copied().unwrap_or(0)
    }

    fn window_value(&self, bot: usize, opponent: usize) -> i64 {
        match (bot, opponent) {
            (_, 0) => self.weight(bot),
            (0, _) => -self.weight(opponent),
            _ => 0,
        }
    }

    fn score_window(&self, counts: WindowCounts, probe: &mut Probe) {
        if counts.off_board > 0 {
            return;
        }
        let win_cells = self.win_length.cells();
        let before = self.window_value(counts.bot, counts.opponent);
        let bot_after = self.window_value(counts.bot + 1, counts.opponent);
        probe.bot.gain = probe.bot.gain.saturating_add(bot_after - before);
        probe.bot.completes_line |= counts.bot + 1 == win_cells;
        let opponent_after = self.window_value(counts.bot, counts.opponent + 1);
        probe.opponent.gain = probe.opponent.gain.saturating_add(before - opponent_after);
        probe.opponent.completes_line |= counts.opponent + 1 == win_cells;
    }

    /// `candidate.probe` must be fresh for the current position.
    pub(crate) fn play(&mut self, candidate: &Candidate, side: Side) {
        self.set_stone(candidate.index, side.stone());
        self.adjust_neighbours(candidate.point, 1);
        self.stone_count += 1;
        self.evaluation = self
            .evaluation
            .saturating_add(candidate.probe.evaluation_delta(side));
    }

    /// Exact inverse of [`Self::play`] with the same candidate.
    pub(crate) fn undo(&mut self, candidate: &Candidate, side: Side) {
        self.set_stone(candidate.index, Stone::Empty);
        self.adjust_neighbours(candidate.point, -1);
        self.stone_count -= 1;
        self.evaluation = self
            .evaluation
            .saturating_sub(candidate.probe.evaluation_delta(side));
    }

    /// Changes only the stone: evaluation and neighbourhoods go stale until the caller restores
    /// it. For the one-ply tactical checks.
    pub(crate) fn set_stone(&mut self, index: usize, stone: Stone) {
        if let Some(cell) = self.stones.get_mut(index) {
            *cell = stone;
        }
    }

    fn adjust_neighbours(&mut self, point: Point, change: i16) {
        self.work_done += NEIGHBOURHOOD_WORK;
        for dy in -2..=2_i32 {
            for dx in -2..=2_i32 {
                let Some(index) = point
                    .offset(dx, dy)
                    .and_then(|cell| self.dimensions.index_of(cell))
                else {
                    continue;
                };
                if let Some(near) = self.stones_within_two.get_mut(index) {
                    *near = near.saturating_add_signed(change);
                }
                if dx.abs() <= 1
                    && dy.abs() <= 1
                    && let Some(near) = self.adjacent_stones.get_mut(index)
                {
                    *near = near.saturating_add_signed(change);
                }
            }
        }
    }

    /// Counts `point` itself as `side`'s whatever it holds.
    fn run_through(&self, point: Point, side: Side, direction: LineDirection) -> Run {
        let mut run = Run {
            length: 1,
            open_ends: 0,
        };
        for step in [-1, 1] {
            let mut distance = step;
            while self.stone_along(point, direction, distance) == side.stone() {
                run.length += 1;
                distance += step;
            }
            if self.stone_along(point, direction, distance) == Stone::Empty {
                run.open_ends += 1;
            }
        }
        run
    }

    pub(crate) fn completes_line(&self, point: Point, side: Side) -> bool {
        LineDirection::ALL.into_iter().any(|direction| {
            self.run_through(point, side, direction).length >= self.win_length.cells()
        })
    }

    /// A run of `win_count - 1` with both ends empty: the opponent can block only one end.
    pub(crate) fn makes_open_threat(&self, point: Point, side: Side) -> bool {
        LineDirection::ALL.into_iter().any(|direction| {
            let run = self.run_through(point, side, direction);
            run.length + 1 >= self.win_length.cells() && run.open_ends == 2
        })
    }

    /// Empty cells where `side` would complete a line using `point`. Each such cell lies on one
    /// of the four lines through `point` within `win_count - 1` steps, so only those are checked.
    pub(crate) fn winning_cells_through(&self, point: Point, side: Side) -> usize {
        let reach = self.win_length.reach();
        let mut count = 0;
        for direction in LineDirection::ALL {
            for steps in (-reach..=reach).filter(|&steps| steps != 0) {
                if self.stone_along(point, direction, steps) != Stone::Empty {
                    continue;
                }
                let Some(cell) = direction.walk(point, steps) else {
                    continue;
                };
                if self.completes_line(cell, side) {
                    count += 1;
                }
            }
        }
        count
    }
}
