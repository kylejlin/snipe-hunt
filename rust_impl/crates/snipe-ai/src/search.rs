use std::cmp::Reverse;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub const MATE_SCORE: i32 = 1_000_000;
const MATE_THRESHOLD: i32 = MATE_SCORE - 10_000;
const INF: i32 = MATE_SCORE + 1;
const MAX_PLY: usize = 128;

/// The narrow contract required from the rules engine.
///
/// One `Move` must be a *complete turn*: snipe step, drop, or both animal
/// steps. Every move therefore changes the player to move. Scores returned by
/// `evaluate` and `terminal_score` are from the current player's perspective.
pub trait GamePosition: Clone {
    type Move: Copy + Debug + Eq + Hash + Ord;

    fn legal_moves(&self, moves: &mut Vec<Self::Move>);
    fn apply_move(&self, mv: Self::Move) -> Self;
    fn position_hash(&self) -> u64;

    /// Return `Some(0)` for a draw, positive for a win, negative for a loss.
    /// Prefer `±MATE_SCORE`; the search normalizes it to prefer faster mates.
    fn terminal_score(&self) -> Option<i32>;
    fn evaluate(&self) -> i32;

    /// Larger values are searched first. Captures, snipe threats, and triplet
    /// activations should receive large bonuses.
    fn move_ordering_score(&self, _mv: Self::Move, _child: &Self) -> i32 {
        0
    }

    /// Whether this move belongs in quiescence search.
    fn is_tactical(&self, _mv: Self::Move, _child: &Self) -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub time_limit: Duration,
    pub max_depth: u8,
    pub quiescence_depth: u8,
    pub transposition_table_mb: usize,
    pub aspiration_window: i32,
    pub deadline_check_interval: u64,
    /// Late-move reduction begins at this zero-based move index.
    pub lmr_after_move: usize,
    /// Maximum ordered moves searched at an ordinary node. Set to zero for
    /// exhaustive search. Snipe Hunt benefits from trading its enormous width
    /// of paired-animal turns for greater tactical depth.
    pub selective_move_limit: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(5),
            max_depth: 64,
            quiescence_depth: 5,
            transposition_table_mb: 32,
            aspiration_window: 80,
            deadline_check_interval: 1_024,
            lmr_after_move: 5,
            selective_move_limit: 48,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchStats {
    pub nodes: u64,
    pub qnodes: u64,
    pub tt_hits: u64,
    pub beta_cutoffs: u64,
    pub researches: u64,
    pub completed_depth: u8,
    pub elapsed: Duration,
}

#[derive(Clone, Debug)]
pub struct SearchResult<M> {
    pub best_move: Option<M>,
    pub score: i32,
    pub depth: u8,
    pub principal_variation: Vec<M>,
    pub stats: SearchStats,
    /// False only when no depth finished (the deterministic fallback is still
    /// legal if `best_move` is `Some`).
    pub completed_iteration: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy, Debug)]
struct Entry<M> {
    key: u64,
    depth: u8,
    score: i32,
    bound: Bound,
    best_move: Option<M>,
    generation: u8,
}

struct TranspositionTable<M> {
    entries: Vec<Option<Entry<M>>>,
    generation: u8,
}

impl<M: Copy> TranspositionTable<M> {
    fn new(megabytes: usize) -> Self {
        let bytes = megabytes.max(1).saturating_mul(1024 * 1024);
        let entry_bytes = std::mem::size_of::<Option<Entry<M>>>().max(1);
        let count = (bytes / entry_bytes).max(1).next_power_of_two() / 2;
        Self {
            entries: vec![None; count.max(1)],
            generation: 0,
        }
    }

    #[inline]
    fn get(&self, key: u64) -> Option<Entry<M>> {
        self.entries[key as usize & (self.entries.len() - 1)].filter(|entry| entry.key == key)
    }

    #[inline]
    fn store(&mut self, entry: Entry<M>) {
        let index = entry.key as usize & (self.entries.len() - 1);
        let replace = match self.entries[index] {
            None => true,
            Some(old) => {
                old.key == entry.key
                    || old.generation != self.generation
                    || entry.depth >= old.depth
            }
        };
        if replace {
            self.entries[index] = Some(entry);
        }
    }

    fn next_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }
}

#[derive(Clone, Copy, Debug)]
struct Timeout;

pub struct SearchEngine<P: GamePosition> {
    config: SearchConfig,
    tt: TranspositionTable<P::Move>,
    history: HashMap<P::Move, i32>,
    killers: Vec<[Option<P::Move>; 2]>,
    stats: SearchStats,
    deadline: Instant,
    path: Vec<u64>,
}

impl<P: GamePosition> SearchEngine<P> {
    pub fn new(config: SearchConfig) -> Self {
        let tt = TranspositionTable::new(config.transposition_table_mb);
        Self {
            config,
            tt,
            history: HashMap::new(),
            killers: vec![[None, None]; MAX_PLY],
            stats: SearchStats::default(),
            deadline: Instant::now(),
            path: Vec::with_capacity(MAX_PLY),
        }
    }

    pub fn config(&self) -> &SearchConfig {
        &self.config
    }

    pub fn set_time_limit(&mut self, time_limit: Duration) {
        self.config.time_limit = time_limit;
    }

    /// Iterative-deepening search. A legal deterministic move is returned even
    /// for a zero-duration budget.
    pub fn search(&mut self, root: &P) -> SearchResult<P::Move> {
        let started = Instant::now();
        self.deadline = started + self.config.time_limit;
        self.stats = SearchStats::default();
        self.path.clear();
        self.tt.next_generation();
        self.decay_history();

        if let Some(score) = root.terminal_score() {
            return SearchResult {
                best_move: None,
                score,
                depth: 0,
                principal_variation: Vec::new(),
                stats: SearchStats {
                    elapsed: started.elapsed(),
                    ..SearchStats::default()
                },
                completed_iteration: true,
            };
        }

        let mut legal = Vec::new();
        root.legal_moves(&mut legal);
        legal.sort_unstable();
        let fallback = legal.first().copied();
        if fallback.is_none() {
            return SearchResult {
                best_move: None,
                score: -MATE_SCORE,
                depth: 0,
                principal_variation: Vec::new(),
                stats: SearchStats {
                    elapsed: started.elapsed(),
                    ..SearchStats::default()
                },
                completed_iteration: true,
            };
        }

        let mut best_move = fallback;
        let mut best_score = root.evaluate();
        let mut completed = false;
        let mut previous_score: i32 = 0;

        for depth in 1..=self.config.max_depth {
            if Instant::now() >= self.deadline {
                break;
            }
            self.path.clear();
            self.path.push(root.position_hash());

            let window = self.config.aspiration_window.max(1);
            let (mut alpha, mut beta) = if depth >= 3 {
                (
                    previous_score.saturating_sub(window),
                    previous_score.saturating_add(window),
                )
            } else {
                (-INF, INF)
            };

            let mut iteration = self.search_root(root, depth, alpha, beta);
            if let Ok((score, _)) = iteration {
                if score <= alpha || score >= beta {
                    self.stats.researches += 1;
                    alpha = -INF;
                    beta = INF;
                    iteration = self.search_root(root, depth, alpha, beta);
                }
            }

            match iteration {
                Ok((score, mv)) => {
                    best_score = score;
                    best_move = Some(mv);
                    previous_score = score;
                    completed = true;
                    self.stats.completed_depth = depth;
                    if score.abs() >= MATE_THRESHOLD {
                        break;
                    }
                }
                Err(Timeout) => break,
            }
        }

        self.stats.elapsed = started.elapsed();
        let pv = if completed {
            self.extract_pv(root, self.stats.completed_depth)
        } else {
            best_move.into_iter().collect()
        };
        SearchResult {
            best_move,
            score: best_score,
            depth: self.stats.completed_depth,
            principal_variation: pv,
            stats: self.stats.clone(),
            completed_iteration: completed,
        }
    }

    fn search_root(
        &mut self,
        root: &P,
        depth: u8,
        mut alpha: i32,
        beta: i32,
    ) -> Result<(i32, P::Move), Timeout> {
        self.check_deadline_force()?;
        let key = root.position_hash();
        let tt_move = self.tt.get(key).and_then(|entry| entry.best_move);
        let mut moves = Vec::new();
        root.legal_moves(&mut moves);
        self.order_moves(root, &mut moves, tt_move, 0);
        self.limit_moves(&mut moves);
        let first = moves[0];
        let original_alpha = alpha;
        let mut best_move = first;
        let mut best_score = -INF;

        for (index, mv) in moves.into_iter().enumerate() {
            let child = root.apply_move(mv);
            let score = if index == 0 {
                -self.negamax(&child, depth - 1, -beta, -alpha, 1)?
            } else {
                let mut score = -self.negamax(&child, depth - 1, -alpha - 1, -alpha, 1)?;
                if score > alpha && score < beta {
                    score = -self.negamax(&child, depth - 1, -beta, -alpha, 1)?;
                }
                score
            };
            if score > best_score {
                best_score = score;
                best_move = mv;
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                self.stats.beta_cutoffs += 1;
                self.record_cutoff(mv, depth, 0);
                break;
            }
        }
        let bound = if best_score <= original_alpha {
            Bound::Upper
        } else if best_score >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt.store(Entry {
            key,
            depth,
            score: score_to_tt(best_score, 0),
            bound,
            best_move: Some(best_move),
            generation: self.tt.generation,
        });
        Ok((best_score, best_move))
    }

    fn negamax(
        &mut self,
        position: &P,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        ply: usize,
    ) -> Result<i32, Timeout> {
        self.stats.nodes += 1;
        self.check_deadline_periodic()?;

        if let Some(score) = position.terminal_score() {
            return Ok(normalize_terminal(score, ply));
        }
        if ply >= MAX_PLY - 1 {
            return Ok(position.evaluate());
        }
        let key = position.position_hash();
        if self.path.contains(&key) {
            return Ok(0);
        }
        if depth == 0 {
            return self.quiescence(position, alpha, beta, self.config.quiescence_depth, ply);
        }

        let original_alpha = alpha;
        let entry = self.tt.get(key);
        if let Some(hit) = entry {
            if hit.depth >= depth {
                self.stats.tt_hits += 1;
                let score = score_from_tt(hit.score, ply);
                match hit.bound {
                    Bound::Exact => return Ok(score),
                    Bound::Lower if score >= beta => return Ok(score),
                    Bound::Upper if score <= alpha => return Ok(score),
                    _ => {}
                }
            }
        }

        let mut moves = Vec::new();
        position.legal_moves(&mut moves);
        if moves.is_empty() {
            // Defensive fallback for adapters that report no-move loss only via
            // legal generation rather than `terminal_score`.
            return Ok(-MATE_SCORE + ply as i32);
        }
        self.order_moves(position, &mut moves, entry.and_then(|e| e.best_move), ply);
        self.limit_moves(&mut moves);
        let mut best = -INF;
        let mut best_move = moves[0];
        self.path.push(key);

        for (index, mv) in moves.into_iter().enumerate() {
            let child = position.apply_move(mv);
            let tactical = position.is_tactical(mv, &child);
            let mut score;

            // PVS plus a conservative one-ply reduction for late quiet moves.
            if index == 0 {
                score = -self.negamax(&child, depth - 1, -beta, -alpha, ply + 1)?;
            } else {
                let reduction =
                    usize::from(depth >= 3 && index >= self.config.lmr_after_move && !tactical)
                        as u8;
                score =
                    -self.negamax(&child, depth - 1 - reduction, -alpha - 1, -alpha, ply + 1)?;
                if score > alpha && reduction != 0 {
                    score = -self.negamax(&child, depth - 1, -alpha - 1, -alpha, ply + 1)?;
                }
                if score > alpha && score < beta {
                    score = -self.negamax(&child, depth - 1, -beta, -alpha, ply + 1)?;
                }
            }

            if score > best {
                best = score;
                best_move = mv;
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                self.stats.beta_cutoffs += 1;
                self.record_cutoff(mv, depth, ply);
                break;
            }
        }
        self.path.pop();

        let bound = if best <= original_alpha {
            Bound::Upper
        } else if best >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.tt.store(Entry {
            key,
            depth,
            score: score_to_tt(best, ply),
            bound,
            best_move: Some(best_move),
            generation: self.tt.generation,
        });
        Ok(best)
    }

    fn quiescence(
        &mut self,
        position: &P,
        mut alpha: i32,
        beta: i32,
        depth: u8,
        ply: usize,
    ) -> Result<i32, Timeout> {
        self.stats.qnodes += 1;
        self.check_deadline_periodic()?;
        if let Some(score) = position.terminal_score() {
            return Ok(normalize_terminal(score, ply));
        }

        let stand_pat = position.evaluate();
        if stand_pat >= beta {
            return Ok(stand_pat);
        }
        alpha = alpha.max(stand_pat);
        if depth == 0 || ply >= MAX_PLY - 1 {
            return Ok(alpha);
        }

        let key = position.position_hash();
        if self.path.contains(&key) {
            return Ok(0);
        }
        let mut tactical = Vec::new();
        let mut legal = Vec::new();
        position.legal_moves(&mut legal);
        for mv in legal {
            let child = position.apply_move(mv);
            if position.is_tactical(mv, &child) {
                tactical.push((mv, child));
            }
        }
        tactical.sort_unstable_by_key(|(mv, child)| {
            Reverse((position.move_ordering_score(*mv, child), Reverse(*mv)))
        });

        self.path.push(key);
        for (_, child) in tactical {
            let score = -self.quiescence(&child, -beta, -alpha, depth - 1, ply + 1)?;
            if score >= beta {
                self.path.pop();
                return Ok(score);
            }
            alpha = alpha.max(score);
        }
        self.path.pop();
        Ok(alpha)
    }

    fn order_moves(
        &self,
        position: &P,
        moves: &mut [P::Move],
        tt_move: Option<P::Move>,
        ply: usize,
    ) {
        let killers = self.killers.get(ply).copied().unwrap_or([None, None]);
        // `sort_unstable_by_key` does not cache keys: its closure may be called
        // O(n log n) times.  A Snipe Hunt ordering key applies the complete
        // turn and evaluates the resulting position, so recomputing it in the
        // comparator consumed most of short search budgets.  Decorate once,
        // sort the cheap tuples, then copy the moves back.
        let mut scored = moves
            .iter()
            .copied()
            .map(|mv| {
                let child = position.apply_move(mv);
                let tactical = position.is_tactical(mv, &child);
                let score = if Some(mv) == tt_move {
                    2_000_000_000
                } else {
                    i32::from(tactical) * 1_000_000
                        + position
                            .move_ordering_score(mv, &child)
                            .clamp(-400_000, 400_000)
                        + i32::from(killers[0] == Some(mv)) * 300_000
                        + i32::from(killers[1] == Some(mv)) * 250_000
                        + self.history.get(&mv).copied().unwrap_or(0).min(200_000)
                };
                (Reverse((score, Reverse(mv))), mv)
            })
            .collect::<Vec<_>>();
        scored.sort_unstable_by_key(|&(key, _)| key);
        for (slot, (_, mv)) in moves.iter_mut().zip(scored) {
            *slot = mv;
        }
    }

    fn record_cutoff(&mut self, mv: P::Move, depth: u8, ply: usize) {
        let bonus = i32::from(depth).pow(2).min(4_096);
        let history = self.history.entry(mv).or_default();
        *history = history.saturating_add(bonus);
        if let Some(killers) = self.killers.get_mut(ply) {
            if killers[0] != Some(mv) {
                killers[1] = killers[0];
                killers[0] = Some(mv);
            }
        }
    }

    fn limit_moves(&self, moves: &mut Vec<P::Move>) {
        if self.config.selective_move_limit != 0 && moves.len() > self.config.selective_move_limit {
            moves.truncate(self.config.selective_move_limit);
        }
    }

    fn decay_history(&mut self) {
        self.history.retain(|_, score| {
            *score /= 2;
            *score != 0
        });
    }

    fn extract_pv(&self, root: &P, depth: u8) -> Vec<P::Move> {
        let mut position = root.clone();
        let mut pv = Vec::with_capacity(depth as usize);
        let mut seen = Vec::with_capacity(depth as usize);
        for _ in 0..depth {
            let key = position.position_hash();
            if seen.contains(&key) {
                break;
            }
            seen.push(key);
            let Some(entry) = self.tt.get(key) else {
                break;
            };
            let Some(mv) = entry.best_move else {
                break;
            };
            let mut legal = Vec::new();
            position.legal_moves(&mut legal);
            if !legal.contains(&mv) {
                break;
            }
            pv.push(mv);
            position = position.apply_move(mv);
            if position.terminal_score().is_some() {
                break;
            }
        }
        pv
    }

    #[inline]
    fn check_deadline_periodic(&self) -> Result<(), Timeout> {
        let interval = self.config.deadline_check_interval.max(1);
        if (self.stats.nodes + self.stats.qnodes)
            .checked_rem(interval)
            .is_some_and(|remainder| remainder == 0)
        {
            self.check_deadline_force()
        } else {
            Ok(())
        }
    }

    #[inline]
    fn check_deadline_force(&self) -> Result<(), Timeout> {
        if Instant::now() >= self.deadline {
            Err(Timeout)
        } else {
            Ok(())
        }
    }
}

#[inline]
fn normalize_terminal(score: i32, ply: usize) -> i32 {
    if score >= MATE_THRESHOLD {
        MATE_SCORE - ply as i32
    } else if score <= -MATE_THRESHOLD {
        -MATE_SCORE + ply as i32
    } else {
        score
    }
}

#[inline]
fn score_to_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_THRESHOLD {
        score + ply as i32
    } else if score <= -MATE_THRESHOLD {
        score - ply as i32
    } else {
        score
    }
}

#[inline]
fn score_from_tt(score: i32, ply: usize) -> i32 {
    if score >= MATE_THRESHOLD {
        score - ply as i32
    } else if score <= -MATE_THRESHOLD {
        score + ply as i32
    } else {
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A take-away game supplies transpositions, tactics and exact mates while
    /// keeping tests independent of the changing rules crate.
    #[derive(Clone)]
    struct TakeAway {
        stones: u8,
    }

    impl GamePosition for TakeAway {
        type Move = u8;

        fn legal_moves(&self, moves: &mut Vec<Self::Move>) {
            for n in 1..=3.min(self.stones) {
                moves.push(n);
            }
        }

        fn apply_move(&self, mv: Self::Move) -> Self {
            Self {
                stones: self.stones - mv,
            }
        }

        fn position_hash(&self) -> u64 {
            self.stones as u64
        }

        fn terminal_score(&self) -> Option<i32> {
            (self.stones == 0).then_some(-MATE_SCORE)
        }

        fn evaluate(&self) -> i32 {
            0
        }

        fn move_ordering_score(&self, mv: Self::Move, _: &Self) -> i32 {
            mv as i32
        }

        fn is_tactical(&self, _: Self::Move, child: &Self) -> bool {
            child.stones == 0
        }
    }

    fn engine(limit: Duration) -> SearchEngine<TakeAway> {
        SearchEngine::new(SearchConfig {
            time_limit: limit,
            max_depth: 16,
            quiescence_depth: 2,
            transposition_table_mb: 1,
            aspiration_window: 20,
            deadline_check_interval: 1,
            lmr_after_move: 5,
            selective_move_limit: 64,
        })
    }

    #[test]
    fn finds_immediate_win() {
        let result = engine(Duration::from_secs(1)).search(&TakeAway { stones: 3 });
        assert_eq!(result.best_move, Some(3));
        assert!(result.score >= MATE_THRESHOLD);
        assert_eq!(result.principal_variation.first(), Some(&3));
    }

    #[test]
    fn solves_forced_game_and_prefers_fast_mate() {
        // Multiples of four are losing. At 6, removing 2 forces a losing 4.
        let result = engine(Duration::from_secs(1)).search(&TakeAway { stones: 6 });
        assert_eq!(result.best_move, Some(2));
        assert!(result.score >= MATE_THRESHOLD);
        assert!(result.stats.nodes > 0);
    }

    #[test]
    fn zero_budget_still_returns_deterministic_legal_fallback() {
        let mut search = engine(Duration::ZERO);
        let a = search.search(&TakeAway { stones: 3 });
        let b = search.search(&TakeAway { stones: 3 });
        assert_eq!(a.best_move, Some(1));
        assert_eq!(a.best_move, b.best_move);
        assert!(!a.completed_iteration);
    }

    #[test]
    fn terminal_position_has_no_move() {
        let result = engine(Duration::from_secs(1)).search(&TakeAway { stones: 0 });
        assert_eq!(result.best_move, None);
        assert_eq!(result.score, -MATE_SCORE);
    }

    #[test]
    fn transposition_table_is_used() {
        let result = engine(Duration::from_secs(1)).search(&TakeAway { stones: 15 });
        assert!(result.stats.tt_hits > 0, "{:?}", result.stats);
    }
}
