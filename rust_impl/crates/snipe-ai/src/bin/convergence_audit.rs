//! Diagnose long, history-aware games that avoid exact repetition without
//! converging to a terminal result.

use std::collections::HashMap;
use std::time::Duration;

use snipe_ai::{evaluate_state, extract_features, SearchConfig, SearchEngine, SearchPolicy};
use snipe_core::{Location, Move, Player, State};

fn main() {
    let seed = argument(1, 2_u64);
    let milliseconds = argument(2, 5_000_u64);
    let max_turns = argument(3, 400_usize);
    let convergence_history_penalty = argument(4, 0_i32);
    let detailed = argument(5, 0_u8) != 0;
    let qsearch_repetition_closures = argument(6, 1_u8) != 0;
    let mut state = State::initial(seed);
    let mut states = vec![state];
    let mut moves_played = Vec::new();
    let mut prior_hashes = Vec::new();
    let mut prior_convergence_hashes = Vec::new();
    let mut exact = HashMap::new();
    let mut canonical = HashMap::<u64, Visits>::new();
    let mut canonical_examples = HashMap::<u64, (usize, State)>::new();
    let mut coarse = HashMap::<u64, Visits>::new();
    let mut features = HashMap::<[i32; 10], Visits>::new();
    let mut move_counts = [0_u64; 3];
    let mut capture_turns = Vec::new();
    let mut snipe_rows = Vec::new();
    let mut recent = Vec::<(usize, Move, u32, i32, u64)>::new();
    let mut alpha_eval_min = i32::MAX;
    let mut alpha_eval_max = i32::MIN;
    let mut turns_played = 0_usize;
    let mut total_nodes = 0_u64;
    let mut total_depth = 0_u64;

    for turn in 0..max_turns {
        if let Some(winner) = state.winner() {
            println!("TERMINAL turn={turn} winner={winner:?}");
            break;
        }
        let hash = state.position_hash();
        if let Some(first) = exact.insert(hash, turn) {
            let preclosing = states[turn - 1];
            let closing_move = moves_played[turn - 1];
            println!(
                "EXACT_REPEAT first={first} turn={turn} cycle_len={} hash={hash:016x} \
repetition_hash={:016x} convergence_hash={:016x}",
                turn - first,
                state.repetition_hash(),
                state.convergence_hash(),
            );
            if detailed {
                println!(
                    "REPEAT_DETAIL preclosing={:?} closing_move={closing_move:?} cycle={:?}",
                    preclosing.to_data(),
                    &moves_played[first..turn],
                );
                probe_closing(preclosing, closing_move, &states[..turn - 1]);
            }
            break;
        }
        let canonical_key = state.repetition_hash();
        if let Some(previous) = canonical.get(&canonical_key) {
            if previous.count == 1 {
                let (first_turn, first_state) = canonical_examples[&canonical_key];
                println!(
                    "CANONICAL_REPEAT first={} turn={turn} span={} key={canonical_key:016x} \
first_state={:?} repeated_state={:?}",
                    previous.first,
                    turn - previous.first,
                    first_state.to_data(),
                    state.to_data(),
                );
                debug_assert_eq!(first_turn, previous.first);
            }
        } else {
            canonical_examples.insert(canonical_key, (turn, state));
        }
        visit(&mut canonical, canonical_key, turn);
        visit(&mut coarse, coarse_signature(state), turn);
        visit(&mut features, feature_signature(state), turn);

        let eval = evaluate_state(state);
        let alpha_eval = if state.side_to_move() == Player::Alpha {
            eval
        } else {
            -eval
        };
        alpha_eval_min = alpha_eval_min.min(alpha_eval);
        alpha_eval_max = alpha_eval_max.max(alpha_eval);
        snipe_rows.push((
            row_number(state.snipe_location(Player::Alpha)),
            row_number(state.snipe_location(Player::Beta)),
        ));
        let config = SearchConfig {
            time_limit: Duration::from_millis(milliseconds),
            max_depth: 3,
            ..SearchConfig::default()
        };
        let policy = SearchPolicy {
            convergence_history_penalty,
            qsearch_repetition_closures,
            ..SearchPolicy::production()
        };
        let mut engine = SearchEngine::new_with_policy(config, policy);
        let result = engine.search_with_context(&state, &prior_hashes, &prior_convergence_hashes);
        total_nodes += result.stats.nodes + result.stats.qnodes;
        total_depth += u64::from(result.depth);
        let mv = result.best_move.expect("nonterminal state has a move");
        moves_played.push(mv);
        move_counts[move_kind(mv)] += 1;
        let outcome = state.apply_move_with_outcome(mv).unwrap();
        let captures = outcome.captured_animals.count_ones() + outcome.captured_snipes.count_ones();
        if captures != 0 {
            capture_turns.push(turn);
        }
        recent.push((turn, mv, captures, alpha_eval, hash));
        if recent.len() > 48 {
            recent.remove(0);
        }
        prior_hashes.push(state.repetition_hash());
        prior_convergence_hashes.push(state.convergence_hash());
        state = outcome.state;
        states.push(state);
        turns_played = turn + 1;

        if (turn + 1) % 50 == 0 {
            print_checkpoint(turn + 1, state, capture_turns.last().copied());
        }
    }

    let repeated_coarse = recurrence_summary(&coarse);
    let repeated_canonical = recurrence_summary(&canonical);
    let repeated_features = recurrence_summary(&features);
    let max_capture_gap = max_event_gap(&capture_turns, turns_played);
    println!(
        "SUMMARY seed={seed} time_ms={milliseconds} turns={turns_played} max_turns={max_turns} \
convergence_penalty={convergence_history_penalty} \
qsearch_repetition={qsearch_repetition_closures} \
moves_animals={} moves_drop={} moves_snipe={} captures={} max_capture_gap={} \
canonical_repeated={} canonical_max_visits={} canonical_max_span={} \
coarse_repeated={} coarse_max_visits={} coarse_max_span={} \
features_repeated={} features_max_visits={} features_max_span={} \
alpha_eval_min={alpha_eval_min} alpha_eval_max={alpha_eval_max}",
        move_counts[0],
        move_counts[1],
        move_counts[2],
        capture_turns.len(),
        max_capture_gap,
        repeated_canonical.repeated,
        repeated_canonical.max_visits,
        repeated_canonical.max_span,
        repeated_coarse.repeated,
        repeated_coarse.max_visits,
        repeated_coarse.max_span,
        repeated_features.repeated,
        repeated_features.max_visits,
        repeated_features.max_span,
    );
    println!(
        "SEARCH avg_nodes={:.1} avg_depth={:.2}",
        total_nodes as f64 / turns_played.max(1) as f64,
        total_depth as f64 / turns_played.max(1) as f64,
    );
    println!(
        "FINAL hash={:016x} legal={} current_can_capture={} state={:?} features={:?}",
        state.position_hash(),
        state.legal_moves().len(),
        state.has_winning_snipe_capture(),
        state.to_data(),
        extract_features(state)
    );
    if detailed {
        println!("CAPTURE_TURNS {capture_turns:?}");
        println!("RECENT {recent:?}");
    }
    println!(
        "SNIPE_SHUTTLES alpha={} beta={}",
        direction_reversals(snipe_rows.iter().map(|rows| rows.0)),
        direction_reversals(snipe_rows.iter().map(|rows| rows.1))
    );
}

fn probe_closing(preclosing: State, closing_move: Move, prior_states: &[State]) {
    let repetition_history = prior_states
        .iter()
        .copied()
        .map(State::repetition_hash)
        .collect::<Vec<_>>();
    let convergence_history = prior_states
        .iter()
        .copied()
        .map(State::convergence_hash)
        .collect::<Vec<_>>();
    for qsearch_repetition_closures in [false, true] {
        let policy = SearchPolicy {
            convergence_history_penalty: 300,
            qsearch_repetition_closures,
            ..SearchPolicy::production()
        };
        let config = SearchConfig {
            time_limit: Duration::from_secs(5),
            max_depth: 3,
            ..SearchConfig::default()
        };
        let mut engine = SearchEngine::new_with_policy(config, policy);
        let result =
            engine.search_with_context(&preclosing, &repetition_history, &convergence_history);
        println!(
            "CLOSING_PROBE qsearch_repetition={qsearch_repetition_closures} original={closing_move:?} \
best={:?} score={} depth={} nodes={}",
            result.best_move,
            result.score,
            result.depth,
            result.stats.nodes + result.stats.qnodes,
        );
    }
}

fn print_checkpoint(turn: usize, state: State, last_capture: Option<usize>) {
    println!(
        "CHECK turn={turn} hash={:016x} eval={} reserve={}:{} owned={}:{} \
snipe={}:{} last_capture={:?}",
        state.position_hash(),
        evaluate_state(state),
        state.reserve_count(Player::Alpha),
        state.reserve_count(Player::Beta),
        owned(state, Player::Alpha),
        owned(state, Player::Beta),
        row_number(state.snipe_location(Player::Alpha)),
        row_number(state.snipe_location(Player::Beta)),
        last_capture,
    );
}

fn owned(state: State, player: Player) -> u32 {
    Location::ALL
        .into_iter()
        .map(|location| state.animal_count(location, player))
        .sum()
}

fn row_number(location: Option<Location>) -> u8 {
    location
        .and_then(Location::row)
        .map_or(0, |row| row.number())
}

fn move_kind(mv: Move) -> usize {
    match mv {
        Move::Animals { .. } => 0,
        Move::Drop { .. } => 1,
        Move::Snipe { .. } => 2,
    }
}

fn coarse_signature(state: State) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut add = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    };
    for location in Location::ALL {
        add(state.animal_count(location, Player::Alpha) as u8);
        add(state.animal_count(location, Player::Beta) as u8);
        add(state.cell(location).snipes());
    }
    add(state.side_to_move() as u8);
    hash
}

fn feature_signature(state: State) -> [i32; 10] {
    let f = extract_features(state);
    [
        f.material,
        f.reserve,
        f.mobility,
        f.progress,
        f.retreaters,
        f.near_triplets,
        f.capture_pressure,
        f.snipe_pressure,
        f.snipe_liberties,
        f.row_freedom,
    ]
}

#[derive(Clone, Copy)]
struct Visits {
    first: usize,
    last: usize,
    count: usize,
}

fn visit<K: Eq + std::hash::Hash>(map: &mut HashMap<K, Visits>, key: K, turn: usize) {
    map.entry(key)
        .and_modify(|visits| {
            visits.last = turn;
            visits.count += 1;
        })
        .or_insert(Visits {
            first: turn,
            last: turn,
            count: 1,
        });
}

struct Recurrence {
    repeated: usize,
    max_visits: usize,
    max_span: usize,
}

fn recurrence_summary<K>(map: &HashMap<K, Visits>) -> Recurrence {
    Recurrence {
        repeated: map.values().filter(|visits| visits.count > 1).count(),
        max_visits: map.values().map(|visits| visits.count).max().unwrap_or(0),
        max_span: map
            .values()
            .map(|visits| visits.last - visits.first)
            .max()
            .unwrap_or(0),
    }
}

fn max_event_gap(events: &[usize], end: usize) -> usize {
    let mut previous = 0;
    let mut max_gap = 0;
    for &event in events {
        max_gap = max_gap.max(event.saturating_sub(previous));
        previous = event;
    }
    max_gap.max(end.saturating_sub(previous))
}

fn direction_reversals(rows: impl Iterator<Item = u8>) -> usize {
    let mut previous = None;
    let mut direction = 0_i8;
    let mut reversals = 0;
    for row in rows {
        if let Some(old) = previous {
            let next = (row as i16 - old as i16).signum() as i8;
            if next != 0 {
                if direction != 0 && next != direction {
                    reversals += 1;
                }
                direction = next;
            }
        }
        previous = Some(row);
    }
    reversals
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
