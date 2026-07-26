//! Banana: a deliberately small, game-specific Snipe Hunt engine.
//!
//! Unlike the generic Almond searcher, Banana keeps the hot path specialized
//! to `snipe_core::State`: terminal checks are constant-time, move ordering
//! reuses already-applied child states, and a narrow, strategic beam spends
//! the budget on depth instead of repeatedly exploring hundreds of equivalent
//! two-animal permutations.

use std::cmp::Reverse;
use std::time::Duration;

use snipe_core::{activates_triplet, Animal, Location, Move, Player, Row, State};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub const MATE_SCORE: i32 = 1_000_000;
const INF: i32 = MATE_SCORE + 1;
const MAX_PLY: usize = 96;
const MAJOR_MASK: u32 =
    (1 << 2) | (1 << 4) | (1 << 12) | (1 << 13) | (1 << 18) | (1 << 20) | (1 << 28) | (1 << 29);

#[derive(Clone, Debug)]
pub struct BananaConfig {
    pub time_limit: Duration,
    pub max_depth: u8,
    pub quiescence_depth: u8,
    pub beam_width: usize,
    pub transposition_table_mb: usize,
    pub deadline_check_interval: u64,
    /// Optional deterministic budget for native strength experiments.
    pub node_limit: Option<u64>,
}

impl Default for BananaConfig {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(3),
            max_depth: 64,
            quiescence_depth: 5,
            beam_width: 48,
            transposition_table_mb: 32,
            deadline_check_interval: 512,
            node_limit: None,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BananaStats {
    pub nodes: u64,
    pub generated_moves: u64,
    pub tt_hits: u64,
    pub beta_cutoffs: u64,
    pub completed_depth: u8,
    pub elapsed: Duration,
}

#[derive(Clone, Debug)]
pub struct BananaResult {
    pub best_move: Option<Move>,
    pub score: i32,
    pub depth: u8,
    pub principal_variation: Vec<Move>,
    pub stats: BananaStats,
    pub completed_iteration: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    key: u64,
    depth: u8,
    score: i32,
    bound: Bound,
    best_move: Move,
    generation: u8,
}

#[derive(Clone, Copy)]
struct Candidate {
    mv: Move,
    child: State,
    ordering: i32,
}

pub struct BananaEngine {
    config: BananaConfig,
    tt: Vec<Option<Entry>>,
    generation: u8,
    deadline: Instant,
    stats: BananaStats,
    active_quiescence_depth: u8,
    path: Vec<u64>,
    history: Vec<u64>,
}

impl BananaEngine {
    pub fn new(config: BananaConfig) -> Self {
        let bytes = config.transposition_table_mb.max(1) * 1024 * 1024;
        let count = (bytes / std::mem::size_of::<Option<Entry>>().max(1))
            .max(2)
            .next_power_of_two()
            / 2;
        Self {
            config,
            tt: vec![None; count],
            generation: 0,
            deadline: Instant::now(),
            stats: BananaStats::default(),
            active_quiescence_depth: 0,
            path: Vec::with_capacity(MAX_PLY),
            history: Vec::new(),
        }
    }

    pub fn set_time_limit(&mut self, time_limit: Duration) {
        self.config.time_limit = time_limit;
    }

    pub fn search(&mut self, root: &State) -> BananaResult {
        self.search_with_history(root, &[])
    }

    pub fn search_with_history(&mut self, root: &State, history: &[u64]) -> BananaResult {
        let started = Instant::now();
        self.deadline = started + self.config.time_limit;
        self.stats = BananaStats::default();
        self.generation = self.generation.wrapping_add(1);
        self.history.clear();
        self.history.extend_from_slice(history);
        self.history.sort_unstable();
        self.history.dedup();

        if let Some(winner) = root.captured_snipe_winner() {
            return BananaResult {
                best_move: None,
                score: if winner == root.side_to_move() {
                    MATE_SCORE
                } else {
                    -MATE_SCORE
                },
                depth: 0,
                principal_variation: Vec::new(),
                stats: self.stats.clone(),
                completed_iteration: true,
            };
        }

        let mut root_moves = Vec::new();
        root.legal_moves_into(&mut root_moves);
        let fallback = root_moves.first().copied();
        if fallback.is_none() {
            return BananaResult {
                best_move: None,
                score: -MATE_SCORE,
                depth: 0,
                principal_variation: Vec::new(),
                stats: self.stats.clone(),
                completed_iteration: true,
            };
        }

        let mut best_move = fallback;
        let mut best_score = evaluate(*root);
        let mut completed = false;
        for depth in 1..=self.config.max_depth {
            if self.out_of_time() {
                break;
            }
            self.path.clear();
            self.path.push(root.repetition_hash());
            self.active_quiescence_depth = if depth == 1 {
                self.config.quiescence_depth.min(1)
            } else {
                self.config.quiescence_depth
            };
            match self.search_root(*root, depth) {
                Ok((score, mv)) => {
                    best_score = score;
                    best_move = Some(mv);
                    completed = true;
                    self.stats.completed_depth = depth;
                    if score.abs() >= MATE_SCORE - MAX_PLY as i32 {
                        break;
                    }
                }
                Err(()) => break,
            }
        }
        self.stats.elapsed = started.elapsed();
        let pv = if completed {
            self.extract_pv(*root, self.stats.completed_depth, best_move)
        } else {
            best_move.into_iter().collect()
        };
        BananaResult {
            best_move,
            score: best_score,
            depth: self.stats.completed_depth,
            principal_variation: pv,
            stats: self.stats.clone(),
            completed_iteration: completed,
        }
    }

    fn search_root(&mut self, root: State, depth: u8) -> Result<(i32, Move), ()> {
        self.check_deadline(true)?;
        let key = root.position_hash();
        let tt_move = self.probe(key).map(|entry| entry.best_move);
        let candidates = self.candidates(root, tt_move, 0)?;
        let Some(first) = candidates.first() else {
            return Err(());
        };
        let mut alpha = -INF;
        let beta = INF;
        let mut best = -INF;
        let mut best_move = first.mv;
        for candidate in candidates {
            let score = -self.negamax(candidate.child, depth - 1, -beta, -alpha, 1)?;
            if score > best {
                best = score;
                best_move = candidate.mv;
            }
            alpha = alpha.max(score);
        }
        self.store(Entry {
            key,
            depth,
            score: best,
            bound: Bound::Exact,
            best_move,
            generation: self.generation,
        });
        Ok((best, best_move))
    }

    fn negamax(
        &mut self,
        state: State,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        ply: usize,
    ) -> Result<i32, ()> {
        self.stats.nodes += 1;
        self.check_deadline(false)?;
        if let Some(winner) = state.captured_snipe_winner() {
            return Ok(if winner == state.side_to_move() {
                MATE_SCORE - ply as i32
            } else {
                -MATE_SCORE + ply as i32
            });
        }
        // This is an exact early-exit query over both animal substeps. It
        // makes one-turn mates visible even at the nominal horizon, avoiding
        // a large capture quiescence tree.
        if state.has_winning_snipe_capture() {
            return Ok(MATE_SCORE - ply as i32 - 1);
        }
        let repetition_key = state.repetition_hash();
        if self.path.contains(&repetition_key)
            || self.history.binary_search(&repetition_key).is_ok()
        {
            return Ok(repetition_score(ply));
        }
        if depth == 0 || ply >= MAX_PLY - 1 {
            return self.quiescence(state, alpha, beta, self.active_quiescence_depth, ply);
        }

        let key = state.position_hash();
        let original_alpha = alpha;
        let hit = self.probe(key);
        if let Some(entry) = hit {
            if entry.depth >= depth {
                self.stats.tt_hits += 1;
                match entry.bound {
                    Bound::Exact => return Ok(entry.score),
                    Bound::Lower if entry.score >= beta => return Ok(entry.score),
                    Bound::Upper if entry.score <= alpha => return Ok(entry.score),
                    _ => {}
                }
            }
        }
        let candidates = self.candidates(state, hit.map(|entry| entry.best_move), ply)?;
        if candidates.is_empty() {
            return Ok(-MATE_SCORE + ply as i32);
        }

        self.path.push(repetition_key);
        let mut best = -INF;
        let mut best_move = candidates[0].mv;
        for (index, candidate) in candidates.into_iter().enumerate() {
            let tactical = is_tactical(state, candidate.child);
            let mut score = if index == 0 {
                -self.negamax(candidate.child, depth - 1, -beta, -alpha, ply + 1)?
            } else {
                let reduction = u8::from(depth >= 3 && index >= 5 && !tactical);
                let mut score = -self.negamax(
                    candidate.child,
                    depth - 1 - reduction,
                    -alpha - 1,
                    -alpha,
                    ply + 1,
                )?;
                if score > alpha && reduction != 0 {
                    score =
                        -self.negamax(candidate.child, depth - 1, -alpha - 1, -alpha, ply + 1)?;
                }
                score
            };
            if index != 0 && score > alpha && score < beta {
                score = -self.negamax(candidate.child, depth - 1, -beta, -alpha, ply + 1)?;
            }
            if score > best {
                best = score;
                best_move = candidate.mv;
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                self.stats.beta_cutoffs += 1;
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
        self.store(Entry {
            key,
            depth,
            score: best,
            bound,
            best_move,
            generation: self.generation,
        });
        Ok(best)
    }

    fn quiescence(
        &mut self,
        state: State,
        mut alpha: i32,
        beta: i32,
        depth: u8,
        ply: usize,
    ) -> Result<i32, ()> {
        if let Some(winner) = state.captured_snipe_winner() {
            return Ok(if winner == state.side_to_move() {
                MATE_SCORE - ply as i32
            } else {
                -MATE_SCORE + ply as i32
            });
        }
        if state.has_winning_snipe_capture() {
            return Ok(MATE_SCORE - ply as i32 - 1);
        }
        let stand_pat = snipe_ai::evaluate_state(state);
        if stand_pat >= beta {
            return Ok(stand_pat);
        }
        alpha = alpha.max(stand_pat);
        if depth == 0 || ply >= MAX_PLY - 1 {
            return Ok(alpha);
        }

        let mover = state.side_to_move();
        let reserve = Location::reserve_of(mover);
        let before_reserve = state.animal_bits(reserve, mover);
        let threatened_snipe_before = presses_snipe(state, mover);
        let mut moves = Vec::new();
        state.legal_moves_into(&mut moves);
        self.stats.generated_moves += moves.len() as u64;
        let mut captures = moves
            .into_iter()
            .filter_map(|mv| {
                let child = state.apply_move(mv).expect("generated move must apply");
                let captured = child.captured_snipe_winner() == Some(mover)
                    || child.animal_bits(reserve, mover) & !before_reserve != 0;
                let creates_snipe_threat = !threatened_snipe_before && presses_snipe(child, mover);
                (captured || creates_snipe_threat).then(|| Candidate {
                    mv,
                    child,
                    ordering: snipe_ai::tactical_move_score(state, mv, child),
                })
            })
            .collect::<Vec<_>>();
        captures
            .sort_unstable_by_key(|candidate| Reverse((candidate.ordering, Reverse(candidate.mv))));
        for candidate in captures {
            self.stats.nodes += 1;
            self.check_deadline(false)?;
            let score = -self.quiescence(candidate.child, -beta, -alpha, depth - 1, ply + 1)?;
            if score >= beta {
                return Ok(score);
            }
            alpha = alpha.max(score);
        }
        Ok(alpha)
    }

    fn candidates(
        &mut self,
        state: State,
        tt_move: Option<Move>,
        ply: usize,
    ) -> Result<Vec<Candidate>, ()> {
        let threatened = opponent_can_capture_snipe(state);
        let mut moves = Vec::new();
        state.legal_moves_into(&mut moves);
        self.stats.generated_moves += moves.len() as u64;
        if let Some(tt_move) = tt_move {
            if let Some(index) = moves.iter().position(|&mv| mv == tt_move) {
                moves.swap(0, index);
            }
        }
        let mut candidates = Vec::with_capacity(moves.len());
        for mv in moves {
            let child = state.apply_move(mv).expect("generated move must apply");
            let mut ordering = move_order(state, mv, child);
            if Some(mv) == tt_move {
                ordering += 2_000_000;
            }
            if threatened {
                // The child belongs to the opponent, so this exact query says
                // whether the candidate actually parries the mate threat.
                if child.has_winning_snipe_capture() {
                    ordering -= 900_000;
                } else {
                    ordering += 900_000;
                }
            }
            candidates.push(Candidate {
                mv,
                child,
                ordering,
            });
        }
        candidates
            .sort_unstable_by_key(|candidate| Reverse((candidate.ordering, Reverse(candidate.mv))));
        let width = if ply == 0 {
            self.config.beam_width.saturating_mul(2)
        } else {
            self.config.beam_width
        };
        if width != 0 {
            candidates.truncate(width.max(1));
        }
        self.check_deadline(false)?;
        Ok(candidates)
    }

    fn probe(&self, key: u64) -> Option<Entry> {
        self.tt[(key as usize) & (self.tt.len() - 1)].filter(|entry| entry.key == key)
    }

    fn store(&mut self, entry: Entry) {
        let slot = (entry.key as usize) & (self.tt.len() - 1);
        if self.tt[slot]
            .is_none_or(|old| old.generation != self.generation || entry.depth >= old.depth)
        {
            self.tt[slot] = Some(entry);
        }
    }

    fn extract_pv(&self, mut state: State, depth: u8, root_move: Option<Move>) -> Vec<Move> {
        let mut pv = Vec::new();
        let mut next = root_move;
        for _ in 0..depth {
            let Some(mv) = next else { break };
            if state.apply_move(mv).is_err() {
                break;
            }
            pv.push(mv);
            state = state.apply_move(mv).expect("move was just validated");
            next = self
                .probe(state.position_hash())
                .map(|entry| entry.best_move);
        }
        pv
    }

    fn out_of_time(&self) -> bool {
        self.config
            .node_limit
            .is_some_and(|limit| self.stats.nodes >= limit)
            || Instant::now() >= self.deadline
    }

    fn check_deadline(&self, force: bool) -> Result<(), ()> {
        let interval = self.config.deadline_check_interval.max(1);
        if (force || self.stats.nodes.is_multiple_of(interval)) && self.out_of_time() {
            Err(())
        } else {
            Ok(())
        }
    }
}

/// Banana's evaluation is intentionally independent of Almond's feature
/// extractor. It encodes the strategy notes directly, while remaining cheap
/// enough to call for every generated child.
pub fn evaluate(state: State) -> i32 {
    // Almond's feature weights have survived substantially more native match
    // testing than the strategy-note terms below. Preserve that empirical
    // signal as the anchor and use the new concepts as a bounded correction.
    let proven = snipe_ai::evaluate_state(state);
    (proven + strategy_evaluate(state) / 4).clamp(-500_000, 500_000)
}

fn strategy_evaluate(state: State) -> i32 {
    let me = state.side_to_move();
    let them = me.opponent();
    let mut score = 0;

    let my_bits = owned_bits(state, me);
    let their_bits = owned_bits(state, them);
    score += (my_bits.count_ones() as i32 - their_bits.count_ones() as i32) * 150;
    score += ((my_bits & MAJOR_MASK).count_ones() as i32
        - (their_bits & MAJOR_MASK).count_ones() as i32)
        * 210;
    score += (state.reserve_count(me) as i32 - state.reserve_count(them) as i32) * 14;

    let my_pressure = pressure_mask(state, me);
    let their_pressure = pressure_mask(state, them);
    score += (my_pressure.count_ones() as i32 - their_pressure.count_ones() as i32) * 72;
    score += ((my_pressure & !their_pressure).count_ones() as i32
        - (their_pressure & !my_pressure).count_ones() as i32)
        * 58;

    score += sanctuary_score(state, me, their_pressure) - sanctuary_score(state, them, my_pressure);
    score += breakthrough_score(state, me) - breakthrough_score(state, them);
    score += development_score(state, me) - development_score(state, them);
    score.clamp(-500_000, 500_000)
}

fn move_order(parent: State, mv: Move, child: State) -> i32 {
    let mover = parent.side_to_move();
    if child.captured_snipe_winner() == Some(mover) {
        return 1_500_000;
    }
    let before = owned_bits(parent, mover);
    let after = owned_bits(child, mover);
    let converted = after & !before;
    let reserve = Location::reserve_of(mover);
    let swept = child.animal_bits(reserve, mover) & !parent.animal_bits(reserve, mover);
    let quiet_shape = -strategy_evaluate(child) / 3;
    let advance = match mv {
        Move::Snipe { .. } => 0,
        Move::Drop { destination, .. } => forward_value(destination, mover) * 8,
        Move::Animals { first, second } => {
            forward_value(first.destination, mover) * 6
                + second.map_or(0, |step| forward_value(step.destination, mover) * 3)
        }
    };
    converted.count_ones() as i32 * 22_000
        + (converted & MAJOR_MASK).count_ones() as i32 * 12_000
        + (swept & !converted).count_ones() as i32 * 1_000
        + quiet_shape
        + advance
}

fn is_tactical(parent: State, child: State) -> bool {
    if child.captured_snipe_winner().is_some() {
        return true;
    }
    let mover = parent.side_to_move();
    let reserve = Location::reserve_of(mover);
    child.animal_bits(reserve, mover) & !parent.animal_bits(reserve, mover) != 0
}

fn owned_bits(state: State, player: Player) -> u32 {
    Location::ALL.into_iter().fold(0, |bits, location| {
        bits | state.animal_bits(location, player)
    })
}

/// Bit N means row N+1 can be activated by this player's animal in one step.
fn pressure_mask(state: State, player: Player) -> u8 {
    let mut mask = 0_u8;
    for row in Row::ALL {
        let destination = state.cell(row.location());
        for source in [row.backward(player), row.forward(player)]
            .into_iter()
            .flatten()
        {
            let source_cell = state.cell(source.location());
            let mut animals = source_cell.animals(player);
            while animals != 0 {
                let index = animals.trailing_zeros() as u8;
                let animal = Animal::from_index(index).expect("set bit is an animal");
                animals &= animals - 1;
                let moves_forward = source.forward(player) == Some(row);
                if (moves_forward || animal.can_retreat())
                    && activates_triplet(destination.all_animals(), animal)
                    && (source_cell.card_count() > 1 || destination.has_snipe(player.opponent()))
                {
                    mask |= 1 << (row.number() - 1);
                    break;
                }
            }
        }
    }
    mask
}

fn opponent_can_capture_snipe(state: State) -> bool {
    let mut data = state.to_data();
    data.side_to_move = state.side_to_move().opponent() as u8;
    data.pending_animal = u8::MAX;
    data.pending_destination = 0;
    State::from_data(data)
        .expect("changing the side to move preserves a full-turn state")
        .has_winning_snipe_capture()
}

fn presses_snipe(state: State, attacker: Player) -> bool {
    state
        .snipe_location(attacker.opponent())
        .and_then(Location::row)
        .is_some_and(|row| pressure_mask(state, attacker) & (1 << (row.number() - 1)) != 0)
}

fn sanctuary_score(state: State, player: Player, enemy_pressure: u8) -> i32 {
    let Some(row) = state.snipe_location(player).and_then(Location::row) else {
        return -400_000;
    };
    let bit = 1 << (row.number() - 1);
    if enemy_pressure & bit != 0 {
        return -950;
    }
    let home = match player {
        Player::Alpha => row.number() <= 3,
        Player::Beta => row.number() >= 4,
    };
    let own_pressure = pressure_mask(state, player);
    let mut trench = 0;
    if row.number() > 1 && own_pressure & (bit >> 1) != 0 {
        trench += 1;
    }
    if row.number() < 6 && own_pressure & (bit << 1) != 0 {
        trench += 1;
    }
    520 + i32::from(home) * 130 + trench * 85
}

fn breakthrough_score(state: State, player: Player) -> i32 {
    let mut score = 0;
    for row in Row::ALL {
        let mut animals = state.animal_bits(row.location(), player);
        while animals != 0 {
            let index = animals.trailing_zeros() as u8;
            let animal = Animal::from_index(index).expect("set bit is an animal");
            animals &= animals - 1;
            if animal.can_retreat() {
                let progress = forward_value(row, player);
                if progress >= 4 {
                    score += (progress - 3) * 34;
                }
            }
        }
    }
    score
}

fn development_score(state: State, player: Player) -> i32 {
    Row::ALL
        .into_iter()
        .map(|row| {
            state.animal_count(row.location(), player) as i32 * forward_value(row, player) * 5
        })
        .sum()
}

fn forward_value(row: Row, player: Player) -> i32 {
    match player {
        Player::Alpha => row.number() as i32,
        Player::Beta => 7 - row.number() as i32,
    }
}

fn repetition_score(ply: usize) -> i32 {
    if ply & 1 == 0 {
        -70_000 + ply as i32
    } else {
        70_000 - ply as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_a_legal_move() {
        let state = State::initial(7);
        let mut engine = BananaEngine::new(BananaConfig {
            time_limit: Duration::from_millis(100),
            max_depth: 2,
            ..BananaConfig::default()
        });
        let result = engine.search(&state);
        assert!(result
            .best_move
            .is_some_and(|mv| state.apply_move(mv).is_ok()));
        assert!(result.depth >= 1);
    }

    #[test]
    fn evaluation_is_antisymmetric_when_the_side_changes() {
        let state = State::initial(11);
        let mut data = state.to_data();
        data.side_to_move = state.side_to_move().opponent() as u8;
        let swapped_turn = State::from_data(data).unwrap();
        assert_eq!(evaluate(state), -evaluate(swapped_turn));
    }

    #[test]
    fn repetition_contempt_is_always_bad_for_the_root() {
        assert!(repetition_score(2) < 0);
        assert!(repetition_score(3) > 0);
        assert!(-repetition_score(3) < 0);
    }
}
