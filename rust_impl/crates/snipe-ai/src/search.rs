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
const REPETITION_CONTEMPT: i32 = 100_000;
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

    /// Position identity used only for repetition detection. Implementations
    /// may canonicalize truly interchangeable piece identities while keeping
    /// `position_hash` exact for the transposition table.
    fn repetition_hash(&self) -> u64 {
        self.position_hash()
    }

    /// Coarse strategic identity for optional soft convergence penalties.
    fn convergence_hash(&self) -> u64 {
        self.repetition_hash()
    }

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

    /// Whether this move creates a new direct attack on the opposing snipe.
    /// Kept separate from `is_tactical` so experiments can extend only the
    /// quiescence frontier without changing production move ordering or LMR.
    fn creates_direct_snipe_threat(&self, _mv: Self::Move, _child: &Self) -> bool {
        false
    }

    /// Whether this is one of the mover's (at most two) snipe-step escapes.
    fn is_snipe_step(&self, _mv: Self::Move) -> bool {
        false
    }

    /// Whether the opponent could capture the mover's snipe in one complete
    /// turn if given the move in the current position.
    fn has_immediate_snipe_capture_threat(&self) -> bool {
        false
    }

    /// Whether the actual side to move can capture the opposing snipe in one
    /// complete turn.
    fn side_to_move_has_snipe_capture(&self) -> bool {
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

/// Search behavior switches used both by production and deterministic A/B
/// arenas. `Default` is the frozen pre-policy baseline; `production()` enables
/// only candidates that passed paired-deal and adversarial validation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SearchPolicy {
    /// Preserve a deeper same-key TT entry when a reduced search later reaches
    /// the position at a shallower depth.
    pub protect_deep_tt_entries: bool,
    /// Include newly-created direct snipe threats in quiescence.
    pub qsearch_direct_snipe_threats: bool,
    /// Include quiet moves in quiescence when their child closes a repetition
    /// against the current search path or supplied game history.
    pub qsearch_repetition_closures: bool,
    /// Keep a fully completed aspiration-window move when only its full-window
    /// verification search runs out of budget.
    pub retain_completed_aspiration_on_timeout: bool,
    /// Reserve beam slots for the mover's (at most two) snipe-step escapes
    /// without disturbing the ordinary PVS ordering.
    pub preserve_critical_snipe_defenses: bool,
    /// Use `GamePosition::repetition_hash` for game-history and path cycles.
    pub canonical_repetition: bool,
    /// Root-relative evaluation penalty per previous visit to a coarse
    /// convergence key. Zero disables this experimental policy.
    pub convergence_history_penalty: i32,
    /// Deterministic per-search work budget used by native policy arenas.
    /// `None` leaves production's deadline-only behavior unchanged.
    pub node_limit: Option<u64>,
}

impl SearchPolicy {
    pub const fn production() -> Self {
        Self {
            protect_deep_tt_entries: false,
            qsearch_direct_snipe_threats: true,
            qsearch_repetition_closures: true,
            retain_completed_aspiration_on_timeout: true,
            preserve_critical_snipe_defenses: true,
            canonical_repetition: true,
            convergence_history_penalty: 300,
            node_limit: None,
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

    fn clear(&mut self) {
        self.entries.fill(None);
    }
}

#[derive(Clone, Copy, Debug)]
struct Timeout;

pub struct SearchEngine<P: GamePosition> {
    config: SearchConfig,
    policy: SearchPolicy,
    tt: TranspositionTable<P::Move>,
    history: HashMap<P::Move, i32>,
    killers: Vec<[Option<P::Move>; 2]>,
    stats: SearchStats,
    deadline: Instant,
    deadline_enabled: bool,
    path: Vec<u64>,
    convergence_path: Vec<u64>,
    prior_hashes: Vec<u64>,
    prior_convergence: HashMap<u64, u16>,
}

impl<P: GamePosition> SearchEngine<P> {
    pub fn new(config: SearchConfig) -> Self {
        Self::new_with_policy(config, SearchPolicy::production())
    }

    pub fn new_with_policy(config: SearchConfig, policy: SearchPolicy) -> Self {
        let tt = TranspositionTable::new(config.transposition_table_mb);
        Self {
            config,
            policy,
            tt,
            history: HashMap::new(),
            killers: vec![[None, None]; MAX_PLY],
            stats: SearchStats::default(),
            deadline: Instant::now(),
            deadline_enabled: true,
            path: Vec::with_capacity(MAX_PLY),
            convergence_path: Vec::with_capacity(MAX_PLY),
            prior_hashes: Vec::new(),
            prior_convergence: HashMap::new(),
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
        self.search_with_history(root, &[])
    }

    /// Search with position hashes from the game timeline strictly before
    /// `root`. Re-entering either a historical position or the current search
    /// path is treated as strongly unfavorable to the root player.
    pub fn search_with_history(&mut self, root: &P, prior_hashes: &[u64]) -> SearchResult<P::Move> {
        self.search_with_context(root, prior_hashes, &[])
    }

    /// Search with both hard repetition keys and coarse strategic history
    /// keys. Coarse keys affect evaluation only when the configured soft
    /// convergence penalty is nonzero.
    pub fn search_with_context(
        &mut self,
        root: &P,
        prior_hashes: &[u64],
        prior_convergence: &[u64],
    ) -> SearchResult<P::Move> {
        self.search_internal(
            root,
            prior_hashes,
            prior_convergence,
            None,
            self.config.max_depth,
            true,
            |_| {},
        )
    }

    /// Iterative-deepening search that stops after `max_depth` completed plies
    /// and reports each newly completed result. `root_moves`, when supplied,
    /// constrains only the first ply; all descendant plies remain unrestricted.
    pub fn search_to_depth_with_context_and_progress<F>(
        &mut self,
        root: &P,
        prior_hashes: &[u64],
        prior_convergence: &[u64],
        root_moves: Option<&[P::Move]>,
        max_depth: u8,
        on_progress: F,
    ) -> SearchResult<P::Move>
    where
        F: FnMut(&SearchResult<P::Move>),
    {
        self.search_internal(
            root,
            prior_hashes,
            prior_convergence,
            root_moves,
            max_depth.max(1),
            false,
            on_progress,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn search_internal<F>(
        &mut self,
        root: &P,
        prior_hashes: &[u64],
        prior_convergence: &[u64],
        root_moves: Option<&[P::Move]>,
        max_depth: u8,
        deadline_enabled: bool,
        mut on_progress: F,
    ) -> SearchResult<P::Move>
    where
        F: FnMut(&SearchResult<P::Move>),
    {
        let started = Instant::now();
        self.deadline = started + self.config.time_limit;
        self.deadline_enabled = deadline_enabled;
        self.stats = SearchStats::default();
        self.path.clear();
        self.convergence_path.clear();
        self.prior_hashes.clear();
        self.prior_hashes.extend_from_slice(prior_hashes);
        self.prior_hashes.sort_unstable();
        self.prior_hashes.dedup();
        self.prior_convergence.clear();
        for &key in prior_convergence {
            let visits = self.prior_convergence.entry(key).or_default();
            *visits = visits.saturating_add(1);
        }
        if !prior_hashes.is_empty() || !prior_convergence.is_empty() {
            // Scores below depend on the supplied game history, so entries
            // retained from a previous root are not valid for this search.
            self.tt.clear();
        }
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
        if let Some(allowed) = root_moves {
            legal.retain(|mv| allowed.contains(mv));
        }
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

        for depth in 1..=max_depth {
            if self.deadline_enabled && Instant::now() >= self.deadline {
                break;
            }
            self.path.clear();
            self.path.push(self.repetition_key(root));
            self.convergence_path.clear();
            self.convergence_path.push(root.convergence_hash());

            let window = self.config.aspiration_window.max(1);
            let (mut alpha, mut beta) = if depth >= 3 {
                (
                    previous_score.saturating_sub(window),
                    previous_score.saturating_add(window),
                )
            } else {
                (-INF, INF)
            };

            let mut iteration = self.search_root(root, root_moves, depth, alpha, beta);
            let mut accepted_aspiration_bound = false;
            if let Ok((score, mv)) = iteration {
                if score <= alpha || score >= beta {
                    self.stats.researches += 1;
                    let completed_bound = (score, mv);
                    alpha = -INF;
                    beta = INF;
                    iteration = self.search_root(root, root_moves, depth, alpha, beta);
                    if iteration.is_err() && self.policy.retain_completed_aspiration_on_timeout {
                        iteration = Ok(completed_bound);
                        accepted_aspiration_bound = true;
                    }
                }
            }

            match iteration {
                Ok((score, mv)) => {
                    best_score = score;
                    best_move = Some(mv);
                    previous_score = score;
                    completed = true;
                    self.stats.completed_depth = depth;
                    self.stats.elapsed = started.elapsed();
                    let progress = SearchResult {
                        best_move,
                        score: best_score,
                        depth,
                        principal_variation: self.extract_pv(root, depth, best_move, best_score),
                        stats: self.stats.clone(),
                        completed_iteration: true,
                    };
                    on_progress(&progress);
                    if score.abs() >= MATE_THRESHOLD {
                        break;
                    }
                    if accepted_aspiration_bound {
                        break;
                    }
                }
                Err(Timeout) => break,
            }
        }

        self.stats.elapsed = started.elapsed();
        let pv = if completed {
            self.extract_pv(root, self.stats.completed_depth, best_move, best_score)
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
        root_moves: Option<&[P::Move]>,
        depth: u8,
        mut alpha: i32,
        beta: i32,
    ) -> Result<(i32, P::Move), Timeout> {
        self.check_deadline_force()?;
        let key = root.position_hash();
        let tt_move = self.tt.get(key).and_then(|entry| entry.best_move);
        let mut moves = Vec::new();
        root.legal_moves(&mut moves);
        if let Some(allowed) = root_moves {
            moves.retain(|mv| allowed.contains(mv));
        }
        self.order_moves(root, &mut moves, tt_move, 0);
        self.limit_moves(root, &mut moves);
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
        self.store_tt(Entry {
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
        let repetition_key = self.repetition_key(position);
        if self.is_repetition(repetition_key) {
            return Ok(repetition_score(ply));
        }
        let convergence_key = position.convergence_hash();
        let convergence_visits = self.convergence_visits(convergence_key);
        if convergence_visits != 0 && self.policy.convergence_history_penalty > 0 {
            return Ok(self.convergence_evaluation(position, ply, convergence_visits));
        }
        if ply >= MAX_PLY - 1 {
            return Ok(self.convergence_evaluation(position, ply, 0));
        }
        if depth == 0 {
            return self.quiescence(position, alpha, beta, self.config.quiescence_depth, ply);
        }

        let key = position.position_hash();
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
        self.limit_moves(position, &mut moves);
        let mut best = -INF;
        let mut best_move = moves[0];
        self.path.push(repetition_key);
        self.convergence_path.push(convergence_key);

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
        self.convergence_path.pop();

        let bound = if best <= original_alpha {
            Bound::Upper
        } else if best >= beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.store_tt(Entry {
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

        let repetition_key = self.repetition_key(position);
        if self.is_repetition(repetition_key) {
            return Ok(repetition_score(ply));
        }
        let convergence_key = position.convergence_hash();
        let convergence_visits = self.convergence_visits(convergence_key);
        if convergence_visits != 0 && self.policy.convergence_history_penalty > 0 {
            return Ok(self.convergence_evaluation(position, ply, convergence_visits));
        }
        let stand_pat = self.convergence_evaluation(position, ply, 0);
        if stand_pat >= beta {
            return Ok(stand_pat);
        }
        alpha = alpha.max(stand_pat);
        if depth == 0 || ply >= MAX_PLY - 1 {
            return Ok(alpha);
        }

        let mut tactical = Vec::new();
        let mut legal = Vec::new();
        position.legal_moves(&mut legal);
        for mv in legal {
            let child = position.apply_move(mv);
            if position.is_tactical(mv, &child)
                || (self.policy.qsearch_direct_snipe_threats
                    && position.creates_direct_snipe_threat(mv, &child))
                || (self.policy.qsearch_repetition_closures
                    && self.is_repetition(self.repetition_key(&child)))
            {
                tactical.push((mv, child));
            }
        }
        tactical.sort_unstable_by_key(|(mv, child)| {
            Reverse((position.move_ordering_score(*mv, child), Reverse(*mv)))
        });

        self.path.push(repetition_key);
        self.convergence_path.push(convergence_key);
        for (_, child) in tactical {
            let score = -self.quiescence(&child, -beta, -alpha, depth - 1, ply + 1)?;
            if score >= beta {
                self.path.pop();
                self.convergence_path.pop();
                return Ok(score);
            }
            alpha = alpha.max(score);
        }
        self.path.pop();
        self.convergence_path.pop();
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

    fn limit_moves(&self, position: &P, moves: &mut Vec<P::Move>) {
        let limit = self.config.selective_move_limit;
        if limit == 0 || moves.len() <= limit {
            return;
        }
        if !self.policy.preserve_critical_snipe_defenses
            || !position.has_immediate_snipe_capture_threat()
        {
            moves.truncate(limit);
            return;
        }

        let escapes = moves[limit..]
            .iter()
            .copied()
            .filter(|&mv| {
                position.is_snipe_step(mv)
                    && !position.apply_move(mv).side_to_move_has_snipe_capture()
            })
            .collect::<Vec<_>>();
        moves.truncate(limit);
        let replace = escapes.len().min(limit);
        for (slot, escape) in moves[limit - replace..].iter_mut().zip(escapes) {
            *slot = escape;
        }
    }

    fn decay_history(&mut self) {
        self.history.retain(|_, score| {
            *score /= 2;
            *score != 0
        });
    }

    #[inline]
    fn is_repetition(&self, key: u64) -> bool {
        self.path.contains(&key) || self.prior_hashes.binary_search(&key).is_ok()
    }

    #[inline]
    fn repetition_key(&self, position: &P) -> u64 {
        if self.policy.canonical_repetition {
            position.repetition_hash()
        } else {
            position.position_hash()
        }
    }

    #[inline]
    fn convergence_visits(&self, key: u64) -> u16 {
        self.prior_convergence
            .get(&key)
            .copied()
            .unwrap_or(0)
            .saturating_add(u16::from(self.convergence_path.contains(&key)))
    }

    #[inline]
    fn convergence_evaluation(&self, position: &P, ply: usize, extra_visits: u16) -> i32 {
        let base = position.evaluate();
        let per_visit = self.policy.convergence_history_penalty.max(0);
        if per_visit == 0 {
            return base;
        }
        let visits = i32::from(
            self.prior_convergence
                .get(&position.convergence_hash())
                .copied()
                .unwrap_or(0)
                .max(extra_visits),
        );
        let root_penalty = per_visit
            .saturating_mul(visits)
            .min(REPETITION_CONTEMPT / 2);
        if ply & 1 == 0 {
            base.saturating_sub(root_penalty)
        } else {
            base.saturating_add(root_penalty)
        }
    }

    #[inline]
    fn store_tt(&mut self, entry: Entry<P::Move>) {
        if !self.policy.protect_deep_tt_entries {
            self.tt.store(entry);
            return;
        }

        let index = entry.key as usize & (self.tt.entries.len() - 1);
        let replace = match self.tt.entries[index] {
            None => true,
            Some(old) if old.key == entry.key => {
                entry.depth > old.depth
                    || (entry.depth == old.depth
                        && (entry.bound == Bound::Exact || old.bound != Bound::Exact))
            }
            Some(old) => old.generation != self.tt.generation || entry.depth >= old.depth,
        };
        if replace {
            self.tt.entries[index] = Some(entry);
        }
    }

    fn extract_pv(
        &self,
        root: &P,
        depth: u8,
        accepted_root_move: Option<P::Move>,
        score: i32,
    ) -> Vec<P::Move> {
        let mate_plies = (score.abs() >= MATE_THRESHOLD)
            .then(|| MATE_SCORE.saturating_sub(score.abs()) as usize);
        let line_length = mate_plies.map_or(depth as usize, |plies| plies.max(depth as usize));
        let mut position = root.clone();
        let mut pv = Vec::with_capacity(line_length);
        let mut seen = Vec::with_capacity(line_length);
        for ply in 0..line_length {
            let key = position.position_hash();
            if seen.contains(&key) {
                break;
            }
            seen.push(key);
            let candidate = if ply == 0 {
                accepted_root_move
            } else {
                self.tt.get(key).and_then(|entry| entry.best_move)
            };
            let mut legal = Vec::new();
            position.legal_moves(&mut legal);
            let mv = candidate.filter(|mv| legal.contains(mv)).or_else(|| {
                // Quiescence does not populate the principal-variation table.
                // When it proves mate just beyond the regular depth, recover
                // the final mating move directly from the penultimate position.
                (mate_plies == Some(ply + 1)).then(|| {
                    legal.iter().copied().find(|mv| {
                        position
                            .apply_move(*mv)
                            .terminal_score()
                            .is_some_and(|s| s < 0)
                    })
                })?
            });
            let Some(mv) = mv else {
                break;
            };
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
        if self
            .policy
            .node_limit
            .is_some_and(|limit| self.stats.nodes + self.stats.qnodes >= limit)
        {
            return Err(Timeout);
        }
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
        if self.deadline_enabled && Instant::now() >= self.deadline {
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
fn repetition_score(ply: usize) -> i32 {
    if ply & 1 == 0 {
        -REPETITION_CONTEMPT
    } else {
        REPETITION_CONTEMPT
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
    fn principal_variation_includes_mate_found_beyond_regular_depth() {
        // At depth one, quiescence sees the opponent's immediate winning
        // capture. The reported mate-in-two line must include that second ply.
        let result = engine(Duration::ZERO).search_to_depth_with_context_and_progress(
            &TakeAway { stones: 4 },
            &[],
            &[],
            None,
            1,
            |_| {},
        );

        assert_eq!(result.score, -MATE_SCORE + 2);
        assert_eq!(result.principal_variation, vec![3, 1]);
    }

    #[test]
    fn depth_search_reports_each_completed_iteration() {
        let mut depths = Vec::new();
        let result = engine(Duration::ZERO).search_to_depth_with_context_and_progress(
            &TakeAway { stones: 20 },
            &[],
            &[],
            None,
            3,
            |progress| depths.push(progress.depth),
        );

        assert_eq!(depths, vec![1, 2, 3]);
        assert_eq!(result.depth, 3);
    }

    #[test]
    fn depth_search_can_constrain_only_the_root_move() {
        let allowed = [1];
        let result = engine(Duration::ZERO).search_to_depth_with_context_and_progress(
            &TakeAway { stones: 20 },
            &[],
            &[],
            Some(&allowed),
            3,
            |_| {},
        );

        assert_eq!(result.best_move, Some(1));
        assert_eq!(result.depth, 3);
        assert!(result.principal_variation.len() > 1);
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

    #[derive(Clone)]
    struct HistoryChoice(u8);

    impl GamePosition for HistoryChoice {
        type Move = u8;

        fn legal_moves(&self, moves: &mut Vec<Self::Move>) {
            if self.0 == 0 {
                moves.extend([0, 1]);
            }
        }

        fn apply_move(&self, mv: Self::Move) -> Self {
            Self(if mv == 0 { 1 } else { 2 })
        }

        fn position_hash(&self) -> u64 {
            match self.0 {
                1 => 42,
                value => u64::from(value),
            }
        }

        fn terminal_score(&self) -> Option<i32> {
            None
        }

        fn evaluate(&self) -> i32 {
            0
        }
    }

    #[test]
    fn historical_repetition_is_unfavorable_to_the_root() {
        let config = SearchConfig {
            time_limit: Duration::from_secs(1),
            max_depth: 1,
            quiescence_depth: 0,
            deadline_check_interval: 1,
            ..SearchConfig::default()
        };
        let mut baseline = SearchEngine::new_with_policy(config.clone(), SearchPolicy::default());
        assert_eq!(baseline.search(&HistoryChoice(0)).best_move, Some(0));

        let mut with_history =
            SearchEngine::new_with_policy(config.clone(), SearchPolicy::default());
        let result = with_history.search_with_history(&HistoryChoice(0), &[99, 42, 42, 7]);
        assert_eq!(result.best_move, Some(1));
        assert!(result.score > -REPETITION_CONTEMPT);

        let convergence_policy = SearchPolicy {
            convergence_history_penalty: 300,
            ..SearchPolicy::default()
        };
        let mut with_convergence = SearchEngine::new_with_policy(config, convergence_policy);
        let result = with_convergence.search_with_context(&HistoryChoice(0), &[], &[42, 42, 99]);
        assert_eq!(result.best_move, Some(1));
    }

    #[derive(Clone)]
    struct QuietClosure(u8);

    impl GamePosition for QuietClosure {
        type Move = u8;

        fn legal_moves(&self, moves: &mut Vec<Self::Move>) {
            if self.0 < 2 {
                moves.push(0);
            }
        }

        fn apply_move(&self, _: Self::Move) -> Self {
            Self(self.0 + 1)
        }

        fn position_hash(&self) -> u64 {
            if self.0 == 2 {
                99
            } else {
                u64::from(self.0)
            }
        }

        fn terminal_score(&self) -> Option<i32> {
            None
        }

        fn evaluate(&self) -> i32 {
            0
        }
    }

    #[test]
    fn quiescence_can_include_quiet_repetition_closures() {
        let config = SearchConfig {
            time_limit: Duration::from_secs(1),
            max_depth: 1,
            quiescence_depth: 1,
            deadline_check_interval: 1,
            ..SearchConfig::default()
        };
        let history = [99];
        let baseline_policy = SearchPolicy {
            qsearch_repetition_closures: false,
            ..SearchPolicy::production()
        };
        let mut baseline = SearchEngine::new_with_policy(config.clone(), baseline_policy);
        let baseline_result = baseline.search_with_context(&QuietClosure(0), &history, &[]);
        assert_eq!(baseline_result.score, 0);

        let policy = SearchPolicy {
            qsearch_repetition_closures: true,
            ..SearchPolicy::production()
        };
        let mut candidate = SearchEngine::new_with_policy(config, policy);
        let candidate_result = candidate.search_with_context(&QuietClosure(0), &history, &[]);
        assert_eq!(candidate_result.score, -REPETITION_CONTEMPT);
        assert!(candidate_result.stats.qnodes > baseline_result.stats.qnodes);
    }

    #[test]
    fn transposition_table_is_used() {
        let result = engine(Duration::from_secs(1)).search(&TakeAway { stones: 15 });
        assert!(result.stats.tt_hits > 0, "{:?}", result.stats);
    }
}
