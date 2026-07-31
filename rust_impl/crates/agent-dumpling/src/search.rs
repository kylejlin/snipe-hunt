use crate::packed::{
    GeneratedMoves, MAX_LINE, PackedMove, PackedState, PackedTurn, TurnList, player_sign,
};
use snipe_core::{
    ActionWriter, Analyzer, Evaluation, EvaluationEstimate, MateInN, OptimalOutcome, Player, State,
};

const MATE: i32 = 1_000_000;
const INFINITY: i32 = 1_100_000;
const NO_BOUND: u8 = 0;
const EXACT: u8 = 1;
const LOWER: u8 = 2;
const UPPER: u8 = 3;
const STATIC_ORDERING_POOL_MULTIPLIER: usize = 4;

#[derive(Clone, Copy)]
pub(crate) struct SearchConfig {
    pub(crate) nodes_per_tick: u32,
    pub(crate) table_bits: u8,
    pub(crate) material_major: i32,
    pub(crate) material_minor: i32,
    pub(crate) alpha_material_minor: i32,
    pub(crate) beta_material_minor: i32,
    pub(crate) reserve_penalty: i32,
    pub(crate) breakthrough: i32,
    pub(crate) infiltration: i32,
    pub(crate) pressure: i32,
    pub(crate) control: i32,
    pub(crate) sanctuary: i32,
    pub(crate) trench: i32,
    pub(crate) snipe_pressure: i32,
    pub(crate) repetition: i32,
    pub(crate) late_move_reduction: bool,
    pub(crate) aspiration_window: i32,
    pub(crate) maximum_full_moves: usize,
    pub(crate) move_count_pruning_depth: i8,
    pub(crate) beta_maximum_full_moves: usize,
    pub(crate) beta_move_count_pruning_depth: i8,
    pub(crate) beta_opening_completed_depth: i8,
    pub(crate) beta_opening_moves: u8,
    pub(crate) capture_threat_per_mille: i32,
    pub(crate) sanctuary_space: i32,
    pub(crate) snipe_safe_exit: i32,
    pub(crate) snipe_support: i32,
    pub(crate) snipe_setup_drop: i32,
    pub(crate) snipe_invader: i32,
    pub(crate) alpha_snipe_invader: i32,
    pub(crate) beta_snipe_invader: i32,
    pub(crate) snipe_near_pressure: i32,
    pub(crate) snipe_near_attacker: i32,
    pub(crate) snipe_home_distance: i32,
    pub(crate) snipe_proximity: i32,
}

#[derive(Clone, Copy, Default)]
struct TableEntry {
    key: u64,
    score: i32,
    best_first: u16,
    best_second: u16,
    depth: i8,
    bound: u8,
    age: u8,
}

pub(crate) struct Searcher {
    config: SearchConfig,
    root: Option<PackedState>,
    root_turns: Box<TurnList>,
    root_winner: Option<Player>,
    score: i32,
    principal_variation: [PackedMove; MAX_LINE],
    principal_variation_len: u8,
    completed_depth: i8,
    target_depth: i8,
    nodes: u32,
    table: Box<[TableEntry]>,
    table_mask: usize,
    history: [i32; 768],
    game_history: [u64; 64],
    game_history_len: u8,
    age: u8,
}

impl Searcher {
    pub(crate) fn new(config: SearchConfig) -> Self {
        let table_len = 1usize << config.table_bits;
        Self {
            config,
            root: None,
            root_turns: Box::new(TurnList::default()),
            root_winner: None,
            score: 0,
            principal_variation: [PackedMove::from_raw(0); MAX_LINE],
            principal_variation_len: 0,
            completed_depth: 0,
            target_depth: 1,
            nodes: 0,
            table: vec![TableEntry::default(); table_len].into_boxed_slice(),
            table_mask: table_len - 1,
            history: [0; 768],
            game_history: [0; 64],
            game_history_len: 0,
            age: 0,
        }
    }

    pub(crate) fn set_state(&mut self, state: State) {
        self.config.material_minor = match state.active_player {
            Player::Alpha => self.config.alpha_material_minor,
            Player::Beta => self.config.beta_material_minor,
        };
        self.config.snipe_invader = match state.active_player {
            Player::Alpha => self.config.alpha_snipe_invader,
            Player::Beta => self.config.beta_snipe_invader,
        };
        self.root_winner = state.winner();
        let root = PackedState::from_core(&state);
        self.root = Some(root);
        let root_hash = root.hash();
        if usize::from(self.game_history_len) == self.game_history.len() {
            self.game_history.copy_within(1.., 0);
            self.game_history_len -= 1;
        }
        self.game_history[usize::from(self.game_history_len)] = root_hash;
        self.game_history_len += 1;
        self.age = self.age.wrapping_add(1);
        self.completed_depth = 0;
        self.target_depth = 1;
        self.principal_variation_len = 0;
        self.score = evaluate(root, self.config);

        if self.root_winner.is_none() {
            let mut turns = TurnList::default();
            root.write_reasonable_turns(&mut turns);
            let unfiltered = turns;
            turns.retain(|turn| {
                let child = root.apply_turn(turn);
                child.captured_winner() == Some(root.active)
                    || !child.active_can_force_snipe_capture_in(3)
            });
            if turns.is_empty() {
                turns = unfiltered;
            }
            *self.root_turns = turns;
            let fallback = if turns.is_empty() {
                root.first_legal_turn()
            } else {
                Some(turns.get(0))
            };
            if let Some(turn) = fallback {
                self.set_root_turn(turn);
            }
        }
    }

    pub(crate) fn think_for_one_tick(&mut self) {
        let Some(root) = self.root else {
            return;
        };
        if self.root_winner.is_some() {
            return;
        }
        let depth_limit = self.depth_limit(root);
        if self.completed_depth >= depth_limit {
            return;
        }

        self.nodes = 0;
        while self.nodes < self.config.nodes_per_tick {
            let depth = self.target_depth;
            let aspiration_alpha = (self.score - self.config.aspiration_window).max(-INFINITY);
            let aspiration_beta = (self.score + self.config.aspiration_window).min(INFINITY);
            let Some(mut score) = self.search(root, depth, aspiration_alpha, aspiration_beta)
            else {
                break;
            };
            if score <= aspiration_alpha || score >= aspiration_beta {
                let Some(full_score) = self.search(root, depth, -INFINITY, INFINITY) else {
                    break;
                };
                score = full_score;
            }
            self.score = score;
            self.completed_depth = depth;
            self.target_depth = self.target_depth.saturating_add(1);
            self.extract_principal_variation();
            if self.completed_depth >= depth_limit {
                break;
            }
            if self.target_depth == i8::MAX {
                break;
            }
        }
    }

    fn search(
        &mut self,
        state: PackedState,
        depth: i8,
        mut alpha: i32,
        mut beta: i32,
    ) -> Option<i32> {
        if self.nodes >= self.config.nodes_per_tick {
            return None;
        }
        self.nodes += 1;

        if let Some(winner) = state.captured_winner() {
            return Some(player_sign(winner) * MATE);
        }

        let key = state.hash();
        if !state.has_leading()
            && Some(state) != self.root
            && self.game_history[..usize::from(self.game_history_len)].contains(&key)
        {
            return Some(
                evaluate(state, self.config) + player_sign(state.active) * self.config.repetition,
            );
        }
        let table_index = key as usize & self.table_mask;
        let cached = self.table[table_index];
        let preferred = (cached.bound != NO_BOUND && cached.key == key)
            .then(|| PackedTurn::from_raw(cached.best_first, cached.best_second));
        let original_alpha = alpha;
        let original_beta = beta;
        if cached.bound != NO_BOUND && cached.key == key && cached.depth >= depth {
            match cached.bound {
                EXACT => return Some(cached.score),
                LOWER => alpha = alpha.max(cached.score),
                UPPER => beta = beta.min(cached.score),
                _ => {}
            }
            if alpha >= beta {
                return Some(cached.score);
            }
        }

        if depth <= 0 {
            return self.quiescence(state, alpha, beta, 2);
        }

        let mut turns = if Some(state) == self.root {
            *self.root_turns
        } else {
            let mut generated = TurnList::default();
            state.write_reasonable_turns(&mut generated);
            generated
        };
        if turns.is_empty() {
            return Some(player_sign(state.active.opponent()) * MATE);
        }
        let maximizing = state.active == Player::Alpha;
        let (maximum_full_moves, move_count_pruning_depth) =
            match self.root.map_or(state.active, |root| root.active) {
                Player::Alpha => (
                    self.config.maximum_full_moves,
                    self.config.move_count_pruning_depth,
                ),
                Player::Beta => (
                    self.config.beta_maximum_full_moves,
                    self.config.beta_move_count_pruning_depth,
                ),
            };
        let selectively_pruned = depth <= move_count_pruning_depth;
        let config = self.config;
        turns.sort_by_scores(|turn| state.turn_order_score(turn, preferred, &self.history));
        if selectively_pruned {
            // A full evaluation includes two-animal snipe-threat probes. Running it
            // over every generated turn made shallow move ordering costlier than
            // the search itself. Preserve all tactically interesting candidates in
            // a cheap first pass, then spend the expensive evaluation only on a
            // compact beam before applying move-count pruning.
            turns.truncate_for_search(
                maximum_full_moves.saturating_mul(STATIC_ORDERING_POOL_MULTIPLIER),
            );
            turns.sort_by_scores(|turn| {
                let tactical = state.turn_order_score(turn, preferred, &self.history);
                let static_score = evaluate(state.apply_turn(turn), config);
                tactical + static_score * if maximizing { 1 } else { -1 }
            });
            turns.truncate_for_search(maximum_full_moves);
        }
        let mut best_score = if maximizing { -INFINITY } else { INFINITY };
        let mut best_turn = turns.get(0);

        for index in 0..turns.len() {
            let turn = turns.get(index);
            let child = state.apply_turn(turn);
            let child_depth = depth - 1;
            let reduction = i8::from(
                self.config.late_move_reduction
                    && depth >= 3
                    && index >= 8
                    && !state.turn_is_forcing(turn),
            );
            let reduced_depth = child_depth - reduction;
            let mut score = if index == 0 {
                self.search(child, reduced_depth, alpha, beta)?
            } else if maximizing {
                self.search(child, reduced_depth, alpha, (alpha + 1).min(beta))?
            } else {
                self.search(child, reduced_depth, (beta - 1).max(alpha), beta)?
            };

            let improves = (maximizing && score > alpha) || (!maximizing && score < beta);
            if reduction != 0 && improves {
                score = if maximizing {
                    self.search(child, child_depth, alpha, (alpha + 1).min(beta))?
                } else {
                    self.search(child, child_depth, (beta - 1).max(alpha), beta)?
                };
            }
            if index != 0 && score > alpha && score < beta {
                score = self.search(child, child_depth, alpha, beta)?;
            }
            if (maximizing && score > best_score) || (!maximizing && score < best_score) {
                best_score = score;
                best_turn = turn;
            }
            if maximizing {
                alpha = alpha.max(score);
            } else {
                beta = beta.min(score);
            }
            if alpha >= beta {
                let bonus = i32::from(depth.max(1)).pow(2).min(4_096);
                for action in [Some(turn.first()), turn.second()].into_iter().flatten() {
                    let history = &mut self.history[action.raw()];
                    *history = (*history + bonus).min(100_000);
                }
                break;
            }
        }

        let bound = if best_score <= original_alpha {
            UPPER
        } else if best_score >= original_beta {
            LOWER
        } else {
            EXACT
        };
        self.store(table_index, key, depth, best_score, bound, best_turn);
        Some(best_score)
    }

    fn quiescence(
        &mut self,
        state: PackedState,
        mut alpha: i32,
        mut beta: i32,
        remaining: u8,
    ) -> Option<i32> {
        if self.nodes >= self.config.nodes_per_tick {
            return None;
        }
        self.nodes += 1;
        if let Some(winner) = state.captured_winner() {
            return Some(player_sign(winner) * MATE);
        }

        let maximizing = state.active == Player::Alpha;
        let in_danger = state.active_snipe_is_pressed();
        let stand_pat = evaluate(state, self.config);
        if remaining == 0 {
            if state.active_has_mating_setup_drop() {
                return Some(player_sign(state.active) * MATE);
            }
            if in_danger {
                let mut evasions = TurnList::default();
                state.write_reasonable_turns(&mut evasions);
                if evasions.is_empty() {
                    return Some(player_sign(state.active.opponent()) * MATE);
                }
            }
            return Some(stand_pat);
        }
        let mut best_score = if in_danger {
            if maximizing { -INFINITY } else { INFINITY }
        } else {
            stand_pat
        };
        if !in_danger {
            if maximizing {
                if best_score >= beta {
                    return Some(best_score);
                }
                alpha = alpha.max(best_score);
            } else {
                if best_score <= alpha {
                    return Some(best_score);
                }
                beta = beta.min(best_score);
            }
        }

        let mut turns = TurnList::default();
        state.write_reasonable_turns(&mut turns);
        if turns.is_empty() {
            return Some(player_sign(state.active.opponent()) * MATE);
        }
        turns.sort_by_scores(|turn| state.turn_order_score(turn, None, &self.history));
        let mut searched = false;
        for index in 0..turns.len() {
            let turn = turns.get(index);
            if !in_danger && !state.turn_is_forcing(turn) {
                continue;
            }
            searched = true;
            let score = self.quiescence(state.apply_turn(turn), alpha, beta, remaining - 1)?;
            if (maximizing && score > best_score) || (!maximizing && score < best_score) {
                best_score = score;
            }
            if maximizing {
                alpha = alpha.max(score);
            } else {
                beta = beta.min(score);
            }
            if alpha >= beta {
                break;
            }
        }
        if in_danger && !searched {
            return Some(player_sign(state.active.opponent()) * MATE);
        }
        Some(best_score)
    }

    fn store(
        &mut self,
        index: usize,
        key: u64,
        depth: i8,
        score: i32,
        bound: u8,
        best: PackedTurn,
    ) {
        let old = self.table[index];
        if old.key != key || old.age != self.age || depth >= old.depth {
            self.table[index] = TableEntry {
                key,
                score,
                best_first: best.first_raw(),
                best_second: best.second_raw(),
                depth,
                bound,
                age: self.age,
            };
        }
    }

    fn depth_limit(&self, root: PackedState) -> i8 {
        if root.active == Player::Beta && self.game_history_len <= self.config.beta_opening_moves {
            self.config.beta_opening_completed_depth
        } else {
            i8::MAX
        }
    }

    fn extract_principal_variation(&mut self) {
        let Some(root) = self.root else {
            return;
        };
        let mut state = root;
        self.principal_variation_len = 0;
        while usize::from(self.principal_variation_len) < MAX_LINE {
            if state.captured_winner().is_some() {
                break;
            }
            let entry = self.table[state.hash() as usize & self.table_mask];
            if entry.bound == NO_BOUND || entry.key != state.hash() {
                break;
            }
            let turn = PackedTurn::from_raw(entry.best_first, entry.best_second);
            let mut turns = TurnList::default();
            state.write_reasonable_turns(&mut turns);
            if !turns.contains(turn) {
                break;
            }
            self.push_turn_to_line(turn);
            state = state.apply_turn(turn);
        }
        if self.principal_variation_len == 0 {
            let turns = *self.root_turns;
            let fallback = if turns.is_empty() {
                root.first_legal_turn()
            } else {
                Some(turns.get(0))
            };
            if let Some(turn) = fallback {
                self.set_root_turn(turn);
            }
        }
        self.complete_root_turn();
    }

    fn set_root_turn(&mut self, turn: PackedTurn) {
        self.principal_variation_len = 0;
        self.push_turn_to_line(turn);
    }

    fn push_turn_to_line(&mut self, turn: PackedTurn) {
        let index = usize::from(self.principal_variation_len);
        if index >= MAX_LINE {
            return;
        }
        self.principal_variation[index] = turn.first();
        self.principal_variation_len += 1;
        if let Some(second) = turn.second()
            && usize::from(self.principal_variation_len) < MAX_LINE
        {
            self.principal_variation[usize::from(self.principal_variation_len)] = second;
            self.principal_variation_len += 1;
        }
    }

    fn complete_root_turn(&mut self) {
        let Some(mut state) = self.root else {
            return;
        };
        let root_player = state.active;
        let mut used = 0usize;
        while used < usize::from(self.principal_variation_len) {
            state = state.apply(self.principal_variation[used]);
            used += 1;
            if state.active != root_player || state.captured_winner().is_some() {
                return;
            }
        }
        while state.active == root_player && state.captured_winner().is_none() && used < MAX_LINE {
            let mut generated = GeneratedMoves::default();
            state.write_reasonable_actions(&mut generated);
            let Some(action) = (!generated.moves.is_empty()).then(|| generated.moves.get(0)) else {
                break;
            };
            self.principal_variation[used] = action;
            self.principal_variation_len = (used + 1) as u8;
            state = state.apply(action);
            used += 1;
        }
    }

    pub(crate) fn evaluation(&self) -> Evaluation {
        if let Some(winner) = self.root_winner {
            return MateInN::new(winner, 0)
                .expect("zero is a valid mate distance")
                .into();
        }
        EvaluationEstimate::from_millipoints(self.score.clamp(-100_000, 100_000))
            .expect("clamped evaluation")
            .into()
    }

    pub(crate) fn fully_solved(&self) -> Option<OptimalOutcome> {
        self.root_winner.map(|winner| {
            OptimalOutcome::MateInN(MateInN::new(winner, 0).expect("zero is a valid mate distance"))
        })
    }

    pub(crate) fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        let Some(mut state) = self.root else {
            return;
        };
        if self.root_winner.is_some() {
            return;
        }
        writer.reserve(usize::from(self.principal_variation_len));
        for index in 0..usize::from(self.principal_variation_len) {
            let action = self.principal_variation[index];
            writer.push(state.to_core_action(action));
            state = state.apply(action);
        }
    }

    pub(crate) fn completed_depth(&self) -> i8 {
        self.completed_depth
    }

    pub(crate) fn nodes_per_tick(&self) -> u32 {
        self.config.nodes_per_tick
    }
}

fn evaluate(state: PackedState, config: SearchConfig) -> i32 {
    let mut score = 0i32;
    const MAJORS: u16 = (1 << 2) | (1 << 4) | (1 << 12) | (1 << 13);
    const RETREATERS: u16 = (1 << 0) | (1 << 3) | (1 << 5) | (1 << 7) | (1 << 11) | (1 << 14);
    const MINORS: u16 = !MAJORS;
    let mut alpha_pressure = [false; 6];
    let mut beta_pressure = [false; 6];
    let mut alpha_rank_material = [0i32; 6];
    let mut beta_rank_material = [0i32; 6];
    for rank in 0..6 {
        alpha_pressure[rank] = state.rank_is_pressed_by(Player::Alpha, rank);
        beta_pressure[rank] = state.rank_is_pressed_by(Player::Beta, rank);
    }

    for (rank, cell) in state.cells.into_iter().enumerate() {
        let alpha_major = cell.category_count(Player::Alpha, MAJORS);
        let beta_major = cell.category_count(Player::Beta, MAJORS);
        let alpha_minor = cell.category_count(Player::Alpha, MINORS);
        let beta_minor = cell.category_count(Player::Beta, MINORS);
        let alpha_value = alpha_major * config.material_major + alpha_minor * config.material_minor;
        let beta_value = beta_major * config.material_major + beta_minor * config.material_minor;
        score += alpha_value - beta_value;
        if rank == 6 {
            score -= (alpha_major + alpha_minor - beta_major - beta_minor) * config.reserve_penalty;
        } else {
            alpha_rank_material[rank] = alpha_value;
            beta_rank_material[rank] = beta_value;
            if rank >= 4 {
                score += cell.category_count(Player::Alpha, RETREATERS) * config.breakthrough;
                score += cell.animal_count(Player::Alpha) as i32 * config.infiltration;
            }
            if rank <= 1 {
                score -= cell.category_count(Player::Beta, RETREATERS) * config.breakthrough;
                score -= cell.animal_count(Player::Beta) as i32 * config.infiltration;
            }
        }
    }

    for rank in 0..6 {
        let alpha_press = alpha_pressure[rank];
        let beta_press = beta_pressure[rank];
        score += (i32::from(alpha_press) - i32::from(beta_press)) * config.pressure;
        if alpha_press && !beta_press {
            score += config.control;
        } else if beta_press && !alpha_press {
            score -= config.control;
        }
        if alpha_press {
            score += beta_rank_material[rank] * config.capture_threat_per_mille / 1_000;
        }
        if beta_press {
            score -= alpha_rank_material[rank] * config.capture_threat_per_mille / 1_000;
        }
    }

    for player in [Player::Alpha, Player::Beta] {
        let Some(snipe_rank) = state.snipe_rank(player) else {
            continue;
        };
        let sign = player_sign(player);
        let (friendly_pressure, enemy_pressure) = match player {
            Player::Alpha => (&alpha_pressure, &beta_pressure),
            Player::Beta => (&beta_pressure, &alpha_pressure),
        };
        let sanctuary = !state.player_can_capture_snipe_this_turn(player.opponent(), player);
        score += sign * i32::from(sanctuary) * config.sanctuary;
        let sanctuary_count = enemy_pressure.iter().filter(|&&pressed| !pressed).count() as i32;
        score += sign * sanctuary_count * config.sanctuary_space;
        score += sign * state.snipe_safe_exit_count(player) as i32 * config.snipe_safe_exit;
        let setup_danger = state.snipe_setup_drop_count(player).min(3) as i32;
        score -= sign * setup_danger * config.snipe_setup_drop;
        score -= sign
            * state.cells[snipe_rank].nonretreater_count(player.opponent())
            * config.snipe_invader;
        score -= sign * state.snipe_near_pressure_count(player) as i32 * config.snipe_near_pressure;
        score -= sign * state.snipe_near_attacker_count(player) as i32 * config.snipe_near_attacker;
        let home_rank = match player {
            Player::Alpha => 0,
            Player::Beta => 5,
        };
        score -= sign * (snipe_rank as i32 - home_rank).abs() * config.snipe_home_distance;
        if let Some(enemy_snipe_rank) = state.snipe_rank(player.opponent()) {
            for rank in 0..6 {
                let distance = (rank as i32 - enemy_snipe_rank as i32).abs();
                let proximity = 5 - distance;
                score += sign
                    * state.cells[rank].animal_count(player) as i32
                    * proximity
                    * config.snipe_proximity;
            }
        }
        let supporting_cards = state.cells[snipe_rank]
            .card_count()
            .saturating_sub(1)
            .min(4) as i32;
        score += sign * supporting_cards * config.snipe_support;
        let left_trench = snipe_rank == 0 || friendly_pressure[snipe_rank - 1];
        let right_trench = snipe_rank == 5 || friendly_pressure[snipe_rank + 1];
        score += sign * i32::from(left_trench && right_trench) * config.trench;
        score += sign
            * i32::from(state.player_can_capture_snipe_this_turn(player, player.opponent()))
            * config.snipe_pressure;
    }

    score + player_sign(state.active) * 8
}

pub(crate) fn analyzer_set_state(searcher: &mut Searcher, state: State) {
    searcher.set_state(state);
}

pub(crate) fn analyzer_think(searcher: &mut Searcher) {
    searcher.think_for_one_tick();
}

pub(crate) fn analyzer_evaluation(searcher: &Searcher) -> Evaluation {
    searcher.evaluation()
}

pub(crate) fn analyzer_fully_solved(searcher: &Searcher) -> Option<OptimalOutcome> {
    searcher.fully_solved()
}

pub(crate) fn analyzer_write_lop<W: ActionWriter>(searcher: &Searcher, writer: &mut W) {
    searcher.write_optimal_lop(writer);
}

pub(crate) fn assert_analyzer<T: Analyzer>() {}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_prng::initial_state;

    #[test]
    fn fixed_work_ticks_make_iterative_deepening_progress() {
        let mut searcher = Searcher::new(SearchConfig {
            nodes_per_tick: 4_096,
            table_bits: 12,
            material_major: 1_200,
            material_minor: 350,
            alpha_material_minor: 350,
            beta_material_minor: 200,
            reserve_penalty: 40,
            breakthrough: 90,
            infiltration: 500,
            pressure: 50,
            control: 80,
            sanctuary: 1_200,
            trench: 180,
            snipe_pressure: 20_000,
            repetition: 20_000,
            late_move_reduction: false,
            aspiration_window: 2_000,
            maximum_full_moves: 24,
            move_count_pruning_depth: 2,
            beta_maximum_full_moves: 12,
            beta_move_count_pruning_depth: 3,
            beta_opening_completed_depth: 4,
            beta_opening_moves: 3,
            capture_threat_per_mille: 750,
            sanctuary_space: 300,
            snipe_safe_exit: 1_000,
            snipe_support: 400,
            snipe_setup_drop: 1_500,
            snipe_invader: 3_000,
            alpha_snipe_invader: 0,
            beta_snipe_invader: 5_000,
            snipe_near_pressure: 1_500,
            snipe_near_attacker: 1_000,
            snipe_home_distance: 500,
            snipe_proximity: 500,
        });
        searcher.set_state(initial_state(7_071));
        for _ in 0..8 {
            searcher.think_for_one_tick();
        }
        assert!(searcher.completed_depth() >= 1);
        let mut line = Vec::new();
        searcher.write_optimal_lop(&mut line);
        assert!(!line.is_empty());
    }
}
