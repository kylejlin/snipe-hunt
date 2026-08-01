use agent_iceberg::IcebergAnalyzer;
use snipe_core::{
    Action, Analyzer, Animal, AnimalDrop, AnimalStep, Evaluation, InitialStateBuilder, Player,
    Rank, SnipeStep, State, StepDirection,
};
use std::{
    env, fs,
    time::{Duration, Instant},
};

fn main() {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let milliseconds = arguments
        .iter()
        .find_map(|value| value.parse::<u64>().ok())
        .unwrap_or(30_000);
    let require_mate = arguments.iter().any(|value| value == "--require-mate");
    let mut state = initial_state();
    let history = fs::read_to_string("../game9.shgh").expect("read game9.shgh");
    for line in history.lines().filter(|line| {
        line.as_bytes().first().is_some_and(u8::is_ascii_digit)
            && !line.starts_with("0a.")
            && !line.starts_with("0b.")
    }) {
        let (label, notation) = line.split_once(". ").expect("turn separator");
        let ply = label
            .trim_end_matches(['a', 'b'])
            .parse::<u32>()
            .expect("numeric ply");
        if ply > 39 {
            continue;
        }
        for step in notation.split(", ") {
            state = state
                .apply(parse_action(step))
                .unwrap_or_else(|error| panic!("illegal `{step}`: {error:?}"));
        }
    }
    assert_eq!(state.active_player, Player::Alpha);

    let mut iceberg = IcebergAnalyzer::new();
    iceberg.set_state(state);
    let started = Instant::now();
    let limit = Duration::from_millis(milliseconds);
    let mut ticks = 0_u64;
    let mut discovered_at = None;
    while started.elapsed() < limit && iceberg.is_fully_solved().is_none() {
        iceberg.think_for_one_tick();
        ticks += 1;
        if discovered_at.is_none() && matches!(iceberg.evaluation(), Evaluation::MateInN(_)) {
            discovered_at = Some(started.elapsed());
        }
    }
    let mut line = Vec::new();
    iceberg.write_optimal_lop(&mut line);
    println!(
        "elapsed={:.3?} discovered={discovered_at:.3?} ticks={ticks} evaluation={:?} solved={:?} actions={}",
        started.elapsed(),
        iceberg.evaluation(),
        iceberg.is_fully_solved(),
        line.len(),
    );
    println!("{}", iceberg.diagnostics());
    println!("line={line:?}");

    if require_mate {
        let Evaluation::MateInN(mate) = iceberg.evaluation() else {
            panic!("Iceberg did not prove a mate within {milliseconds} ms");
        };
        assert_eq!(mate.winner(), Player::Alpha);
        let initiating_ply = [
            Action::AnimalStep(AnimalStep {
                actor: Animal::Dog,
                direction: StepDirection::Advance,
                destination: Rank::R4,
            }),
            Action::AnimalStep(AnimalStep {
                actor: Animal::Ox,
                direction: StepDirection::Advance,
                destination: Rank::R5,
            }),
        ];
        assert!(
            line.len() >= 2
                && initiating_ply
                    .iter()
                    .all(|action| line[..2].contains(action)),
            "Iceberg missed the forcing ply-40 attack: {line:?}",
        );
    }
}

fn initial_state() -> State {
    InitialStateBuilder {
        alpha_reserve: [Animal::Mouse],
        r1: [Animal::Rooster, Animal::Fish],
        r2: [
            Animal::Ox,
            Animal::Ox,
            Animal::Tiger,
            Animal::Rabbit,
            Animal::Dragon,
            Animal::Snake,
            Animal::Snake,
            Animal::Ram,
            Animal::Monkey,
            Animal::Elephant,
            Animal::Frog,
            Animal::Frog,
        ],
        r3: [Animal::Dog],
        r4: [Animal::Mouse],
        r5: [
            Animal::Rabbit,
            Animal::Horse,
            Animal::Horse,
            Animal::Ram,
            Animal::Monkey,
            Animal::Rooster,
            Animal::Dog,
            Animal::Boar,
            Animal::Boar,
            Animal::Elephant,
            Animal::Squid,
            Animal::Squid,
        ],
        r6: [Animal::Dragon, Animal::Fish],
        beta_reserve: [Animal::Tiger],
    }
    .build()
    .expect("valid initial position")
}

fn parse_action(notation: &str) -> Action {
    let clean = notation
        .trim_end_matches("+#0")
        .trim_end_matches("-#0")
        .trim_end_matches('x');
    let (name, movement) = clean.split_once(' ').expect("piece and destination");
    let destination = movement
        .bytes()
        .find(|byte| (b'1'..=b'6').contains(byte))
        .map(|byte| rank(byte - b'0'))
        .unwrap();
    if name == "Alpha" || name == "Beta" {
        return Action::SnipeStep(SnipeStep { destination });
    }
    let actor = animal(name);
    if movement.starts_with('&') {
        Action::Drop(AnimalDrop { actor, destination })
    } else {
        Action::AnimalStep(AnimalStep {
            actor,
            direction: if movement.starts_with('*') {
                StepDirection::Retreat
            } else {
                StepDirection::Advance
            },
            destination,
        })
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

fn rank(value: u8) -> Rank {
    match value {
        1 => Rank::R1,
        2 => Rank::R2,
        3 => Rank::R3,
        4 => Rank::R4,
        5 => Rank::R5,
        6 => Rank::R6,
        _ => unreachable!(),
    }
}
