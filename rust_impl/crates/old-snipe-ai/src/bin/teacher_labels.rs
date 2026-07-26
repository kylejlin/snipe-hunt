//! Mine positions where the bounded production search disagrees with a
//! substantially deeper instance of the same engine.
//!
//! This is a diagnostic, not a trainer.  It prints exact, replayable states
//! and evaluation features so a proposed search/evaluation correction can be
//! derived from TRAIN records and checked once against the held-out seeds.

use std::cmp::Reverse;
use std::time::Duration;

use snipe_ai::{
    extract_features, SearchConfig, SearchEngine, SearchPolicy, SearchResult, SnipeFeatures,
    MATE_SCORE,
};
use snipe_core::{Move, Player, State, StateData};

#[derive(Clone, Debug)]
struct Mistake {
    seed: u64,
    ply: usize,
    split: Split,
    state: StateData,
    side: Player,
    legal_moves: usize,
    root_features: SnipeFeatures,
    fast_move: Move,
    fast_score: i32,
    fast_depth: u8,
    fast_nodes: u64,
    fast_pv: Vec<Move>,
    teacher_move: Move,
    teacher_score: i32,
    teacher_depth: u8,
    teacher_nodes: u64,
    teacher_pv: Vec<Move>,
    fast_move_teacher_value: i32,
    teacher_move_teacher_value: i32,
    fast_child_depth: u8,
    teacher_child_depth: u8,
    fast_child_features: SnipeFeatures,
    teacher_child_features: SnipeFeatures,
}

impl Mistake {
    fn regret(&self) -> i32 {
        self.teacher_move_teacher_value - self.fast_move_teacher_value
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Split {
    Train,
    Holdout,
}

#[derive(Default)]
struct SplitSummary {
    sampled: u64,
    completed: u64,
    teacher_deeper: u64,
    disagreements: u64,
    forced_incomparable: u64,
    positive_regret: u64,
    total_positive_regret: i64,
}

impl SplitSummary {
    fn print(&self, label: &str) {
        println!(
            "SUMMARY split={label} sampled={} completed={} teacher_deeper={} disagreements={} \
forced_incomparable={} positive_regret={} avg_positive_regret={:.1}",
            self.sampled,
            self.completed,
            self.teacher_deeper,
            self.disagreements,
            self.forced_incomparable,
            self.positive_regret,
            if self.positive_regret == 0 {
                0.0
            } else {
                self.total_positive_regret as f64 / self.positive_regret as f64
            }
        );
    }
}

fn main() {
    if std::env::args().any(|argument| matches!(argument.as_str(), "-h" | "--help")) {
        usage();
        return;
    }

    let seeds = argument(1, 2_u64);
    let first_seed = argument(2, 0_u64);
    let max_plies = argument(3, 24_usize);
    let sample_stride = argument(4, 6_usize).max(1);
    let fast_nodes = argument(5, 20_000_u64).max(1);
    let teacher_nodes = argument(6, 200_000_u64).max(fast_nodes + 1);
    let records_to_print = argument(7, 12_usize);
    let holdout_modulus = argument(8, 5_u64).max(2);

    let mut fast = engine(fast_nodes);
    let mut teacher = engine(teacher_nodes);
    let mut fast_child_teacher = engine(teacher_nodes);
    let mut best_child_teacher = engine(teacher_nodes);
    let mut train = SplitSummary::default();
    let mut holdout = SplitSummary::default();
    let mut mistakes = Vec::new();

    println!(
        "CONFIG seeds={seeds} first_seed={first_seed} max_plies={max_plies} \
sample_stride={sample_stride} fast_nodes={fast_nodes} teacher_nodes={teacher_nodes} \
holdout_rule=seed_mod_{holdout_modulus}_equals_0"
    );

    for seed in first_seed..first_seed.saturating_add(seeds) {
        let split = if seed % holdout_modulus == 0 {
            Split::Holdout
        } else {
            Split::Train
        };
        let summary = match split {
            Split::Train => &mut train,
            Split::Holdout => &mut holdout,
        };
        let mut state = State::initial(seed);
        let mut repetition_history = Vec::new();
        let mut convergence_history = Vec::new();

        for ply in 0..max_plies {
            if state.winner().is_some() {
                break;
            }
            let fast_result =
                fast.search_with_context(&state, &repetition_history, &convergence_history);
            let fast_move = fast_result
                .best_move
                .expect("a nonterminal state must have a legal move");

            if ply % sample_stride == 0 {
                summary.sampled += 1;
                let teacher_result =
                    teacher.search_with_context(&state, &repetition_history, &convergence_history);
                let teacher_move = teacher_result
                    .best_move
                    .expect("a nonterminal state must have a legal move");
                let both_completed =
                    fast_result.completed_iteration && teacher_result.completed_iteration;
                summary.completed += u64::from(both_completed);
                let teacher_is_deeper = both_completed
                    && (teacher_result.depth > fast_result.depth
                        || teacher_result.score.abs() >= MATE_SCORE - 10_000);
                summary.teacher_deeper += u64::from(teacher_is_deeper);

                if teacher_is_deeper && teacher_move != fast_move {
                    summary.disagreements += 1;
                    let mut child_repetition = repetition_history.clone();
                    child_repetition.push(state.repetition_hash());
                    let mut child_convergence = convergence_history.clone();
                    child_convergence.push(state.convergence_hash());
                    let fast_child = state
                        .apply_move(fast_move)
                        .expect("search-selected move must apply");
                    let best_child = state
                        .apply_move(teacher_move)
                        .expect("search-selected move must apply");
                    let fast_child_result = fast_child_teacher.search_with_context(
                        &fast_child,
                        &child_repetition,
                        &child_convergence,
                    );
                    let best_child_result = best_child_teacher.search_with_context(
                        &best_child,
                        &child_repetition,
                        &child_convergence,
                    );
                    let fast_value = -fast_child_result.score;
                    let best_value = -best_child_result.score;
                    let regret = best_value - fast_value;
                    let forced_comparable = fast_child_result.completed_iteration
                        && best_child_result.completed_iteration
                        && (fast_child_result.depth == best_child_result.depth
                            || fast_child_result.score.abs() >= MATE_SCORE - 10_000
                            || best_child_result.score.abs() >= MATE_SCORE - 10_000);
                    summary.forced_incomparable += u64::from(!forced_comparable);
                    if forced_comparable && regret > 0 {
                        summary.positive_regret += 1;
                        summary.total_positive_regret += i64::from(regret);
                        // Holdout records remain blind: only their aggregate
                        // rates are emitted, preventing feature-level tuning
                        // against the test split.
                        if split == Split::Train {
                            let legal = state.legal_moves();
                            mistakes.push(Mistake {
                                seed,
                                ply,
                                split,
                                state: state.to_data(),
                                side: state.side_to_move(),
                                legal_moves: legal.len(),
                                root_features: extract_features(state),
                                fast_move,
                                fast_score: fast_result.score,
                                fast_depth: fast_result.depth,
                                fast_nodes: searched_nodes(&fast_result),
                                fast_pv: fast_result.principal_variation,
                                teacher_move,
                                teacher_score: teacher_result.score,
                                teacher_depth: teacher_result.depth,
                                teacher_nodes: searched_nodes(&teacher_result),
                                teacher_pv: teacher_result.principal_variation,
                                fast_move_teacher_value: fast_value,
                                teacher_move_teacher_value: best_value,
                                fast_child_depth: fast_child_result.depth,
                                teacher_child_depth: best_child_result.depth,
                                fast_child_features: root_perspective_features(fast_child),
                                teacher_child_features: root_perspective_features(best_child),
                            });
                        }
                    }
                }
            }

            repetition_history.push(state.repetition_hash());
            convergence_history.push(state.convergence_hash());
            state = state
                .apply_move(fast_move)
                .expect("search-selected move must apply");
        }
    }

    mistakes.sort_by_key(|mistake| {
        (
            Reverse(mistake.regret()),
            mistake.seed,
            mistake.ply,
            mistake.fast_move,
        )
    });
    for (rank, mistake) in mistakes.iter().take(records_to_print).enumerate() {
        print_mistake(rank + 1, mistake);
    }
    train.print("TRAIN");
    holdout.print("HOLDOUT");
    println!(
        "RESULT train_labels={} printed={} methodology=\"derive hypotheses from TRAIN only; \
freeze candidate; measure HOLDOUT regret/disagreement and paired arena win rate once\"",
        mistakes.len(),
        mistakes.len().min(records_to_print)
    );
}

fn engine(node_limit: u64) -> SearchEngine<State> {
    SearchEngine::new_with_policy(
        SearchConfig {
            time_limit: Duration::from_secs(60),
            max_depth: 64,
            transposition_table_mb: 64,
            deadline_check_interval: 1,
            ..SearchConfig::default()
        },
        SearchPolicy {
            node_limit: Some(node_limit),
            ..SearchPolicy::production()
        },
    )
}

fn searched_nodes(result: &SearchResult<Move>) -> u64 {
    result.stats.nodes + result.stats.qnodes
}

fn root_perspective_features(child: State) -> SnipeFeatures {
    negate_features(extract_features(child))
}

fn negate_features(features: SnipeFeatures) -> SnipeFeatures {
    SnipeFeatures {
        material: -features.material,
        major_material: -features.major_material,
        reserve: -features.reserve,
        mobility: -features.mobility,
        progress: -features.progress,
        retreaters: -features.retreaters,
        near_triplets: -features.near_triplets,
        capture_pressure: -features.capture_pressure,
        snipe_pressure: -features.snipe_pressure,
        snipe_liberties: -features.snipe_liberties,
        row_freedom: -features.row_freedom,
    }
}

fn print_mistake(rank: usize, mistake: &Mistake) {
    println!(
        "MISTAKE rank={rank} split={:?} seed={} ply={} side={:?} legal={} regret={} \
fast_move={:?} fast_root_score={} fast_depth={} fast_nodes={} \
teacher_move={:?} teacher_root_score={} teacher_depth={} teacher_nodes={} \
fast_move_teacher_value={} teacher_move_teacher_value={} \
forced_child_depths={}/{}",
        mistake.split,
        mistake.seed,
        mistake.ply,
        mistake.side,
        mistake.legal_moves,
        mistake.regret(),
        mistake.fast_move,
        mistake.fast_score,
        mistake.fast_depth,
        mistake.fast_nodes,
        mistake.teacher_move,
        mistake.teacher_score,
        mistake.teacher_depth,
        mistake.teacher_nodes,
        mistake.fast_move_teacher_value,
        mistake.teacher_move_teacher_value,
        mistake.fast_child_depth,
        mistake.teacher_child_depth,
    );
    println!("  STATE {:?}", mistake.state);
    println!(
        "  FEATURES root={:?} after_fast={:?} after_teacher={:?} \
delta_fast={:?} delta_teacher={:?}",
        mistake.root_features,
        mistake.fast_child_features,
        mistake.teacher_child_features,
        feature_delta(mistake.fast_child_features, mistake.root_features),
        feature_delta(mistake.teacher_child_features, mistake.root_features),
    );
    println!(
        "  PV fast_root_matches={} fast={:?} teacher_root_matches={} teacher={:?}",
        mistake.fast_pv.first().copied() == Some(mistake.fast_move),
        mistake.fast_pv,
        mistake.teacher_pv.first().copied() == Some(mistake.teacher_move),
        mistake.teacher_pv,
    );
}

fn feature_delta(after: SnipeFeatures, before: SnipeFeatures) -> SnipeFeatures {
    SnipeFeatures {
        material: after.material - before.material,
        major_material: after.major_material - before.major_material,
        reserve: after.reserve - before.reserve,
        mobility: after.mobility - before.mobility,
        progress: after.progress - before.progress,
        retreaters: after.retreaters - before.retreaters,
        near_triplets: after.near_triplets - before.near_triplets,
        capture_pressure: after.capture_pressure - before.capture_pressure,
        snipe_pressure: after.snipe_pressure - before.snipe_pressure,
        snipe_liberties: after.snipe_liberties - before.snipe_liberties,
        row_freedom: after.row_freedom - before.row_freedom,
    }
}

fn usage() {
    println!(
        "usage: cargo run -p old-snipe-ai --release --bin teacher_labels -- \
<seeds=2> <first_seed=0> <max_plies=24> <sample_stride=6> \
<fast_nodes=20000> <teacher_nodes=200000> <records=12> <holdout_modulus=5>\n\
\n\
Seeds divisible by holdout_modulus are blind HOLDOUT; all others are TRAIN. \
Only disagreements where the teacher completed a deeper iteration (or found \
mate) and equal-depth forced-move reanalysis assigns positive regret become \
labels. Exact records are printed only for TRAIN; HOLDOUT emits aggregates."
    );
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
