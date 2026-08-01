use agent_garlic::GarlicAnalyzer;
use agent_honey::HoneyAnalyzer;
use snipe_core::{
    Action, Analyzer, Animal, AnimalDrop, AnimalStep, Evaluation, Player, Rank, SnipeStep, State,
    StepDirection,
};
use snipe_prng::initial_state;
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const CHERRY_FAJITA_0: &str = include_str!(
    "../../../agent-arena-results/round-robin-acf-20260730/cherry-vs-fajita/seed-00000000000000000000-cherry-as-alpha.shgh"
);
const CHERRY_FAJITA_7: &str = include_str!(
    "../../../agent-arena-results/round-robin-acf-20260730/cherry-vs-fajita/seed-00000000000000000007-cherry-as-beta.shgh"
);
const CHERRY_FAJITA_5: &str = include_str!(
    "../../../agent-arena-results/round-robin-acf-20260730/cherry-vs-fajita/seed-00000000000000000005-cherry-as-beta.shgh"
);
const CHERRY_FAJITA_6: &str = include_str!(
    "../../../agent-arena-results/round-robin-acf-20260730/cherry-vs-fajita/seed-00000000000000000006-cherry-as-beta.shgh"
);
const CHERRY_FAJITA_9: &str = include_str!(
    "../../../agent-arena-results/round-robin-acf-20260730/cherry-vs-fajita/seed-00000000000000000009-cherry-as-beta.shgh"
);

struct Case {
    name: &'static str,
    history: &'static str,
    plies_before_end: usize,
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    let milliseconds = arguments
        .windows(2)
        .find(|pair| pair[0] == "--milliseconds")
        .map(|pair| pair[1].as_str())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(1_000);
    let filter = arguments
        .windows(2)
        .find(|pair| pair[0] == "--filter")
        .map(|pair| pair[1].as_str());
    let honey_only = arguments.iter().any(|argument| argument == "--honey-only");
    if arguments.iter().any(|argument| argument == "--scan") {
        scan_histories(Duration::from_millis(milliseconds));
        return;
    }
    let time_limit = Duration::from_millis(milliseconds);
    let cases = [
        Case {
            name: "cherry-fajita-0/end-4",
            history: CHERRY_FAJITA_0,
            plies_before_end: 4,
        },
        Case {
            name: "cherry-fajita-7/end-4",
            history: CHERRY_FAJITA_7,
            plies_before_end: 4,
        },
        Case {
            name: "cherry-fajita-5/end-8",
            history: CHERRY_FAJITA_5,
            plies_before_end: 8,
        },
        Case {
            name: "cherry-fajita-6/end-6",
            history: CHERRY_FAJITA_6,
            plies_before_end: 6,
        },
        Case {
            name: "cherry-fajita-9/end-6",
            history: CHERRY_FAJITA_9,
            plies_before_end: 6,
        },
        Case {
            name: "cherry-fajita-0/end-8",
            history: CHERRY_FAJITA_0,
            plies_before_end: 8,
        },
        Case {
            name: "cherry-fajita-7/end-8",
            history: CHERRY_FAJITA_7,
            plies_before_end: 8,
        },
        Case {
            name: "cherry-fajita-0/end-20",
            history: CHERRY_FAJITA_0,
            plies_before_end: 20,
        },
        Case {
            name: "cherry-fajita-7/end-20",
            history: CHERRY_FAJITA_7,
            plies_before_end: 20,
        },
        Case {
            name: "cherry-fajita-0/end-10",
            history: CHERRY_FAJITA_0,
            plies_before_end: 10,
        },
        Case {
            name: "cherry-fajita-7/end-10",
            history: CHERRY_FAJITA_7,
            plies_before_end: 10,
        },
    ];

    println!("Honey mate workload: {milliseconds} ms per agent and position");
    println!("case                              agent    ticks     elapsed   evaluation   solved");
    for case in cases
        .into_iter()
        .filter(|case| filter.is_none_or(|filter| case.name.contains(filter)))
    {
        let state = position_before_end(case.history, case.plies_before_end);
        run_honey(case.name, state.clone(), time_limit);
        if !honey_only {
            run(
                case.name,
                "Garlic",
                GarlicAnalyzer::new(),
                state,
                time_limit,
            );
        }
    }
}

fn run_honey(name: &str, state: State, limit: Duration) {
    let mut analyzer = HoneyAnalyzer::new();
    analyzer.set_state(state);
    let start = Instant::now();
    let mut ticks = 0_u64;
    let mut slowest_tick = Duration::ZERO;
    while start.elapsed() < limit && analyzer.is_fully_solved().is_none() {
        let tick_start = Instant::now();
        analyzer.think_for_one_tick();
        slowest_tick = slowest_tick.max(tick_start.elapsed());
        ticks += 1;
    }
    println!(
        "{name:<33} {agent:<8} {ticks:>8} {elapsed:>10.3?} {evaluation:>12?}   {solved}",
        agent = "Honey",
        elapsed = start.elapsed(),
        evaluation = analyzer.evaluation(),
        solved = analyzer.is_fully_solved().is_some(),
    );
    println!(
        "  slowest_tick={slowest_tick:.3?} {}",
        analyzer.diagnostics()
    );
}

fn scan_histories(limit: Duration) {
    let mut paths = Vec::new();
    let history_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../agent-arena-results");
    collect_histories(&history_root, &mut paths);
    paths.retain(|path| is_strong_history(path));
    paths.sort();
    println!(
        "Scanning {} strong-agent histories at {:?} per position",
        paths.len(),
        limit
    );
    println!("mate  recorded-offset  elapsed   max-tick    garlic      history");
    for path in paths {
        let Ok(history) = fs::read_to_string(&path) else {
            continue;
        };
        if !history
            .lines()
            .last()
            .is_some_and(|line| line.contains("#0"))
        {
            continue;
        }
        let turn_count = history
            .lines()
            .filter(|line| {
                line.as_bytes().first().is_some_and(u8::is_ascii_digit)
                    && !line.starts_with("0a.")
                    && !line.starts_with("0b.")
            })
            .count();
        for offset in [6, 8, 10, 12] {
            if turn_count < offset {
                continue;
            }
            let state = position_before_end(&history, offset);
            let mut honey = HoneyAnalyzer::new();
            honey.set_state(state);
            let start = Instant::now();
            let mut slowest_tick = Duration::ZERO;
            while start.elapsed() < limit && honey.is_fully_solved().is_none() {
                let tick_start = Instant::now();
                honey.think_for_one_tick();
                slowest_tick = slowest_tick.max(tick_start.elapsed());
            }
            let honey_elapsed = start.elapsed();
            if let Evaluation::MateInN(mate) = honey.evaluation()
                && mate.plies() >= 6
            {
                let mut garlic = GarlicAnalyzer::new();
                garlic.set_state(position_before_end(&history, offset));
                let garlic_start = Instant::now();
                while garlic_start.elapsed() < limit && garlic.is_fully_solved().is_none() {
                    garlic.think_for_one_tick();
                }
                println!(
                    "{:>4} {:>16} {:>9.3?} {:>9.3?} {:>10?}    {}",
                    mate.plies(),
                    offset,
                    honey_elapsed,
                    slowest_tick,
                    garlic.evaluation(),
                    path.display()
                );
            }
        }
    }
}

fn is_strong_history(path: &Path) -> bool {
    let Some(pairing) = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
    else {
        return false;
    };
    matches!(
        pairing,
        "cherry-vs-fajita" | "cherry-vs-garlic" | "fajita-vs-garlic"
    )
}

fn collect_histories(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_histories(&path, paths);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "shgh")
        {
            paths.push(path);
        }
    }
}

fn run<A: Analyzer>(name: &str, agent: &str, mut analyzer: A, state: State, limit: Duration) {
    analyzer.set_state(state);
    let start = Instant::now();
    let mut ticks = 0_u64;
    while start.elapsed() < limit && analyzer.is_fully_solved().is_none() {
        analyzer.think_for_one_tick();
        ticks += 1;
    }
    let elapsed = start.elapsed();
    println!(
        "{name:<33} {agent:<8} {ticks:>8} {elapsed:>10.3?} {evaluation:>12?}   {solved}",
        evaluation = analyzer.evaluation(),
        solved = analyzer.is_fully_solved().is_some(),
    );
}

fn position_before_end(history: &str, plies_before_end: usize) -> State {
    let seed = history
        .lines()
        .find_map(|line| line.strip_prefix("// Seed: "))
        .and_then(|seed| seed.parse::<u64>().ok())
        .expect("history has a numeric seed");
    let turns = history
        .lines()
        .filter(|line| {
            line.as_bytes().first().is_some_and(u8::is_ascii_digit)
                && !line.starts_with("0a.")
                && !line.starts_with("0b.")
        })
        .collect::<Vec<_>>();
    assert!(turns.len() >= plies_before_end);
    let mut state = initial_state(seed);
    for turn in &turns[..turns.len() - plies_before_end] {
        let (_, notation) = turn.split_once(". ").expect("turn separator");
        for step in notation.split(", ") {
            let action = parse_action(&state, step);
            state = state
                .apply(action)
                .unwrap_or_else(|error| panic!("illegal history action `{step}`: {error:?}"));
        }
    }
    assert_eq!(state.winner(), None);
    state
}

fn parse_action(state: &State, notation: &str) -> Action {
    let clean = notation
        .trim_end_matches("+#0")
        .trim_end_matches("-#0")
        .trim_end_matches('x');
    let (name, movement) = clean
        .split_once(' ')
        .expect("action has a piece and destination");
    let destination = movement
        .bytes()
        .find(|byte| (b'1'..=b'6').contains(byte))
        .map(|byte| number_rank(byte - b'0'))
        .expect("action has an in-range destination");
    if name == "Alpha" || name == "Beta" {
        return Action::SnipeStep(SnipeStep { destination });
    }
    let actor = animal(name);
    if movement.starts_with('&') {
        Action::Drop(AnimalDrop { actor, destination })
    } else {
        let direction = if movement.starts_with('*') {
            StepDirection::Retreat
        } else {
            StepDirection::Advance
        };
        let action = Action::AnimalStep(AnimalStep {
            actor,
            direction,
            destination,
        });
        debug_assert!(state.clone().apply(action).is_ok());
        action
    }
}

fn animal(name: &str) -> Animal {
    match name {
        "Rat" => Animal::Mouse,
        "Ox" => Animal::Ox,
        "Tiger" => Animal::Tiger,
        "Rabbit" => Animal::Rabbit,
        "Dragon" => Animal::Dragon,
        "Snake" => Animal::Snake,
        "Horse" => Animal::Horse,
        "Ram" => Animal::Ram,
        "Monkey" => Animal::Monkey,
        "Rooster" => Animal::Rooster,
        "Dog" => Animal::Dog,
        "Boar" => Animal::Boar,
        "Fish" => Animal::Fish,
        "Elephant" => Animal::Elephant,
        "Squid" => Animal::Squid,
        "Frog" => Animal::Frog,
        _ => panic!("unknown animal `{name}`"),
    }
}

fn number_rank(number: u8) -> Rank {
    match number {
        1 => Rank::R1,
        2 => Rank::R2,
        3 => Rank::R3,
        4 => Rank::R4,
        5 => Rank::R5,
        6 => Rank::R6,
        _ => unreachable!(),
    }
}

#[allow(dead_code)]
fn winner(evaluation: Evaluation) -> Option<Player> {
    match evaluation {
        Evaluation::MateInN(mate) => Some(mate.winner()),
        Evaluation::Estimate(_) => None,
    }
}
