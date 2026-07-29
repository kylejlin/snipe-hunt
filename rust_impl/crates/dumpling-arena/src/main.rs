mod history;

use agent_avocado::AvocadoAnalyzer;
use agent_blueberry::BlueberryAnalyzer;
use agent_cherry::CherryAnalyzer;
use agent_dumpling::v1::DumplingV1Analyzer;
use snipe_core::{Action, Analyzer, Evaluation, Player, State};
use snipe_prng::initial_state;
use std::{
    env, fs,
    path::PathBuf,
    process::ExitCode,
    time::{Duration, Instant},
};

use crate::history::HistoryRecorder;

const DEFAULT_PAIRS: u64 = 10;
const DEFAULT_CHALLENGER_MS: u64 = 5_000;
const DEFAULT_OLDER_MS: u64 = 10_000;
const DEFAULT_MAX_PLIES: u32 = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AgentKind {
    V1,
    Avocado,
    Blueberry,
    Cherry,
}

impl AgentKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "v1" | "dumpling-v1" => Ok(Self::V1),
            "avocado" => Ok(Self::Avocado),
            "blueberry" => Ok(Self::Blueberry),
            "cherry" => Ok(Self::Cherry),
            _ => Err(format!("unknown agent `{value}`")),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::V1 => "Dumpling v1",
            Self::Avocado => "Avocado",
            Self::Blueberry => "Blueberry",
            Self::Cherry => "Cherry",
        }
    }

    fn create(self) -> ArenaAgent {
        match self {
            Self::V1 => ArenaAgent::V1(Box::new(DumplingV1Analyzer::new())),
            Self::Avocado => ArenaAgent::Avocado(AvocadoAnalyzer::new()),
            Self::Blueberry => ArenaAgent::Blueberry(BlueberryAnalyzer::new()),
            Self::Cherry => ArenaAgent::Cherry(CherryAnalyzer::new()),
        }
    }
}

enum ArenaAgent {
    V1(Box<DumplingV1Analyzer>),
    Avocado(AvocadoAnalyzer),
    Blueberry(BlueberryAnalyzer),
    Cherry(CherryAnalyzer),
}

impl ArenaAgent {
    fn set_state(&mut self, state: State) {
        match self {
            Self::V1(agent) => agent.set_state(state),
            Self::Avocado(agent) => agent.set_state(state),
            Self::Blueberry(agent) => agent.set_state(state),
            Self::Cherry(agent) => agent.set_state(state),
        }
    }

    fn think_for_one_tick(&mut self) {
        match self {
            Self::V1(agent) => agent.think_for_one_tick(),
            Self::Avocado(agent) => agent.think_for_one_tick(),
            Self::Blueberry(agent) => agent.think_for_one_tick(),
            Self::Cherry(agent) => agent.think_for_one_tick(),
        }
    }

    fn evaluation(&self) -> Evaluation {
        match self {
            Self::V1(agent) => agent.evaluation(),
            Self::Avocado(agent) => agent.evaluation(),
            Self::Blueberry(agent) => agent.evaluation(),
            Self::Cherry(agent) => agent.evaluation(),
        }
    }

    fn line(&self) -> Vec<Action> {
        let mut line = Vec::new();
        match self {
            Self::V1(agent) => agent.write_optimal_lop(&mut line),
            Self::Avocado(agent) => agent.write_optimal_lop(&mut line),
            Self::Blueberry(agent) => agent.write_optimal_lop(&mut line),
            Self::Cherry(agent) => agent.write_optimal_lop(&mut line),
        }
        line
    }

    fn depth(&self) -> Option<i8> {
        match self {
            Self::V1(agent) => Some(agent.completed_depth()),
            Self::Avocado(agent) => i8::try_from(agent.completed_depth()).ok(),
            _ => None,
        }
    }
}

struct Config {
    challenger: AgentKind,
    opponent: AgentKind,
    pairs: u64,
    challenger_time: Duration,
    older_time: Duration,
    seed_start: u64,
    max_plies: u32,
    trace: bool,
    trace_state: bool,
    side: Option<Player>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Winner(Player),
    Draw,
}

struct GameReport {
    outcome: Outcome,
    plies: u32,
    alpha_ticks: u64,
    beta_ticks: u64,
    history: String,
}

#[derive(Clone, Copy)]
struct Trace {
    moves: bool,
    state: bool,
}

fn main() -> ExitCode {
    match parse_args().and_then(run) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("dumpling-arena: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args() -> Result<Config, String> {
    let mut config = Config {
        challenger: AgentKind::V1,
        opponent: AgentKind::Avocado,
        pairs: DEFAULT_PAIRS,
        challenger_time: Duration::from_millis(DEFAULT_CHALLENGER_MS),
        older_time: Duration::from_millis(DEFAULT_OLDER_MS),
        seed_start: 0,
        max_plies: DEFAULT_MAX_PLIES,
        trace: false,
        trace_state: false,
        side: None,
    };
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            return Err(usage().to_owned());
        }
        if flag == "--trace" {
            config.trace = true;
            continue;
        }
        if flag == "--trace-state" {
            config.trace_state = true;
            continue;
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for `{flag}`\n{}", usage()))?;
        match flag.as_str() {
            "--challenger" => config.challenger = AgentKind::parse(&value)?,
            "--opponent" => config.opponent = AgentKind::parse(&value)?,
            "--pairs" => config.pairs = parse(&flag, &value)?,
            "--challenger-ms" => {
                config.challenger_time = Duration::from_millis(parse(&flag, &value)?)
            }
            "--older-ms" => config.older_time = Duration::from_millis(parse(&flag, &value)?),
            "--seed-start" => config.seed_start = parse(&flag, &value)?,
            "--max-plies" => config.max_plies = parse(&flag, &value)?,
            "--side" => {
                config.side = Some(match value.to_ascii_lowercase().as_str() {
                    "alpha" => Player::Alpha,
                    "beta" => Player::Beta,
                    _ => return Err("`--side` must be `alpha` or `beta`".to_owned()),
                })
            }
            _ => return Err(format!("unknown option `{flag}`\n{}", usage())),
        }
    }
    if config.pairs == 0 {
        return Err("`--pairs` must be positive".to_owned());
    }
    Ok(config)
}

fn parse<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value `{value}` for `{flag}`"))
}

fn usage() -> &'static str {
    "usage: dumpling-arena [--challenger v1] [--opponent avocado|blueberry|cherry|v1] \
     [--pairs 10] [--challenger-ms 5000] [--older-ms 10000] [--seed-start 0] \
     [--max-plies 256] [--side alpha|beta] [--trace] [--trace-state]"
}

fn run(config: Config) -> Result<bool, String> {
    println!(
        "challenge: {} ({:.3}s/ply) vs {} ({:.3}s/ply), {} paired seeds",
        config.challenger.label(),
        config.challenger_time.as_secs_f64(),
        config.opponent.label(),
        config.older_time.as_secs_f64(),
        config.pairs,
    );
    let started = Instant::now();
    let mut wins = 0u64;

    for pair in 0..config.pairs {
        let seed = config.seed_start.wrapping_add(pair);
        for challenger_side in [Player::Alpha, Player::Beta]
            .into_iter()
            .filter(|side| config.side.is_none_or(|selected| selected == *side))
        {
            let alpha_kind = if challenger_side == Player::Alpha {
                config.challenger
            } else {
                config.opponent
            };
            let beta_kind = if challenger_side == Player::Beta {
                config.challenger
            } else {
                config.opponent
            };
            let alpha_time = if challenger_side == Player::Alpha {
                config.challenger_time
            } else {
                config.older_time
            };
            let beta_time = if challenger_side == Player::Beta {
                config.challenger_time
            } else {
                config.older_time
            };
            let report = play_game(
                seed,
                alpha_kind,
                beta_kind,
                alpha_time,
                beta_time,
                config.max_plies,
                Trace {
                    moves: config.trace,
                    state: config.trace_state,
                },
            )?;
            if is_dumpling_cherry(alpha_kind, beta_kind)
                && matches!(report.outcome, Outcome::Winner(_))
            {
                let path = save_history(&report, alpha_kind, beta_kind)?;
                println!("saved history: {}", path.display());
            }
            println!(
                "seed {seed:>4}, challenger {:>5}: {:?} in {} plies \
                 (ticks alpha={}, beta={}, elapsed={:.1}s)",
                format!("{challenger_side:?}"),
                report.outcome,
                report.plies,
                report.alpha_ticks,
                report.beta_ticks,
                started.elapsed().as_secs_f64(),
            );
            if report.outcome != Outcome::Winner(challenger_side) {
                println!(
                    "FAILED immediately after game {}: challenger did not win",
                    wins + 1
                );
                return Ok(false);
            }
            wins += 1;
        }
    }
    println!(
        "PASSED: {} won all {wins} games in {:.1}s",
        config.challenger.label(),
        started.elapsed().as_secs_f64()
    );
    Ok(true)
}

fn play_game(
    seed: u64,
    alpha_kind: AgentKind,
    beta_kind: AgentKind,
    alpha_time: Duration,
    beta_time: Duration,
    max_plies: u32,
    trace: Trace,
) -> Result<GameReport, String> {
    let mut state = initial_state(seed);
    let mut alpha = alpha_kind.create();
    let mut beta = beta_kind.create();
    let mut alpha_ticks = 0u64;
    let mut beta_ticks = 0u64;
    let mut history = HistoryRecorder::new(
        &state,
        seed,
        alpha_kind.label(),
        beta_kind.label(),
        alpha_time,
        beta_time,
    );

    for ply in 0..max_plies {
        if let Some(winner) = state.winner() {
            return Ok(GameReport {
                outcome: Outcome::Winner(winner),
                plies: ply,
                alpha_ticks,
                beta_ticks,
                history: history.finish(),
            });
        }
        let player = state.active_player;
        let (agent, budget, ticks) = match player {
            Player::Alpha => (&mut alpha, alpha_time, &mut alpha_ticks),
            Player::Beta => (&mut beta, beta_time, &mut beta_ticks),
        };
        agent.set_state(state.clone());
        let started = Instant::now();
        loop {
            agent.think_for_one_tick();
            *ticks += 1;
            if started.elapsed() >= budget {
                break;
            }
        }
        let evaluation = agent.evaluation();
        let line = agent.line();
        if line.is_empty() {
            return Err(format!(
                "{} returned an empty line in a live position (evaluation {evaluation:?})",
                match player {
                    Player::Alpha => alpha_kind.label(),
                    Player::Beta => beta_kind.label(),
                }
            ));
        }
        if trace.moves {
            println!(
                "  ply {ply:>3} {player:?} {} depth={:?} eval={evaluation:?} move={:?}",
                match player {
                    Player::Alpha => alpha_kind.label(),
                    Player::Beta => beta_kind.label(),
                },
                agent.depth(),
                line.iter().take(2).collect::<Vec<_>>(),
            );
        }
        if trace.state {
            println!("  before: {state:?}");
        }

        let turn_start = state.clone();
        let mut completed = false;
        let mut played = Vec::with_capacity(2);
        for action in line {
            played.push(action);
            state = state.apply(action).map_err(|error| {
                format!(
                    "{} returned illegal action {action:?}: {error:?}",
                    match player {
                        Player::Alpha => alpha_kind.label(),
                        Player::Beta => beta_kind.label(),
                    }
                )
            })?;
            if state.active_player != player || state.winner().is_some() {
                completed = true;
                break;
            }
        }
        history.record_turn(ply + 1, &turn_start, &played)?;
        if !completed {
            return Err(format!(
                "{} returned an incomplete animal turn",
                match player {
                    Player::Alpha => alpha_kind.label(),
                    Player::Beta => beta_kind.label(),
                }
            ));
        }
    }

    Ok(GameReport {
        outcome: Outcome::Draw,
        plies: max_plies,
        alpha_ticks,
        beta_ticks,
        history: history.finish(),
    })
}

fn is_dumpling_cherry(alpha: AgentKind, beta: AgentKind) -> bool {
    matches!(
        (alpha, beta),
        (AgentKind::V1, AgentKind::Cherry) | (AgentKind::Cherry, AgentKind::V1)
    )
}

fn save_history(report: &GameReport, alpha: AgentKind, beta: AgentKind) -> Result<PathBuf, String> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join("dumpling_v0_vs_cherry");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let mut next_game = 1u64;
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read history entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(number) = name
            .strip_prefix("game")
            .and_then(|rest| rest.split_once('('))
            .and_then(|(number, _)| number.parse::<u64>().ok())
        {
            next_game = next_game.max(number + 1);
        }
    }
    let winner = match report.outcome {
        Outcome::Winner(Player::Alpha) if alpha == AgentKind::V1 => "dumpling_won",
        Outcome::Winner(Player::Beta) if beta == AgentKind::V1 => "dumpling_won",
        Outcome::Winner(_) => "cherry_won",
        Outcome::Draw => "draw",
    };
    let path = directory.join(format!("game{next_game}({winner}).shgh"));
    fs::write(&path, &report.history)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}
