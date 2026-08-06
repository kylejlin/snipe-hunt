mod history;

use agent_avocado::AvocadoAnalyzer;
use agent_cherry::CherryAnalyzer;
use agent_fajita::FajitaAnalyzer;
use agent_garlic::GarlicAnalyzer;
use agent_iceberg::IcebergAnalyzer;
use agent_kiwi::KiwiAnalyzer;
use snipe_core::{Action, Analyzer, Evaluation, Player, State};
use snipe_prng::initial_state;
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::history::HistoryRecorder;

const DEFAULT_PAIRS: u64 = 10;
const DEFAULT_MILLISECONDS: u64 = 10_000;
const DEFAULT_MAX_PLIES: u32 = 256;
const DEFAULT_OUTPUT_ROOT: &str = "agent-arena-results";
const DEFAULT_AGENTS: [AgentKind; 4] = [
    AgentKind::Cherry,
    AgentKind::Garlic,
    AgentKind::Fajita,
    AgentKind::Kiwi,
];

type Matchup = (AgentKind, AgentKind);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum AgentKind {
    Avocado,
    Cherry,
    Fajita,
    Garlic,
    Iceberg,
    Kiwi,
}

impl AgentKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Avocado => "Avocado",
            Self::Cherry => "Cherry",
            Self::Fajita => "Fajita",
            Self::Garlic => "Garlic",
            Self::Iceberg => "Iceberg",
            Self::Kiwi => "Kiwi",
        }
    }

    const fn slug(self) -> &'static str {
        match self {
            Self::Avocado => "avocado",
            Self::Cherry => "cherry",
            Self::Fajita => "fajita",
            Self::Garlic => "garlic",
            Self::Iceberg => "iceberg",
            Self::Kiwi => "kiwi",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "avocado" => Ok(Self::Avocado),
            "cherry" => Ok(Self::Cherry),
            "fajita" => Ok(Self::Fajita),
            "garlic" => Ok(Self::Garlic),
            "iceberg" => Ok(Self::Iceberg),
            "kiwi" => Ok(Self::Kiwi),
            _ => Err(format!(
                "unknown agent `{value}`; expected avocado, cherry, fajita, garlic, iceberg, or kiwi"
            )),
        }
    }

    fn create(self) -> ArenaAgent {
        match self {
            Self::Avocado => ArenaAgent::Avocado(AvocadoAnalyzer::new()),
            Self::Cherry => ArenaAgent::Cherry(CherryAnalyzer::new()),
            Self::Fajita => ArenaAgent::Fajita(FajitaAnalyzer::new()),
            Self::Garlic => ArenaAgent::Garlic(GarlicAnalyzer::new()),
            Self::Iceberg => ArenaAgent::Iceberg(Box::new(IcebergAnalyzer::new())),
            Self::Kiwi => ArenaAgent::Kiwi(KiwiAnalyzer::new()),
        }
    }
}

enum ArenaAgent {
    Avocado(AvocadoAnalyzer),
    Cherry(CherryAnalyzer),
    Fajita(FajitaAnalyzer),
    Garlic(GarlicAnalyzer),
    Iceberg(Box<IcebergAnalyzer>),
    Kiwi(KiwiAnalyzer),
}

impl ArenaAgent {
    fn set_state(&mut self, state: State) {
        match self {
            Self::Avocado(agent) => agent.set_state(state),
            Self::Cherry(agent) => agent.set_state(state),
            Self::Fajita(agent) => agent.set_state(state),
            Self::Garlic(agent) => agent.set_state(state),
            Self::Iceberg(agent) => agent.set_state(state),
            Self::Kiwi(agent) => agent.set_state(state),
        }
    }

    fn think_for_one_tick(&mut self) {
        match self {
            Self::Avocado(agent) => agent.think_for_one_tick(),
            Self::Cherry(agent) => agent.think_for_one_tick(),
            Self::Fajita(agent) => agent.think_for_one_tick(),
            Self::Garlic(agent) => agent.think_for_one_tick(),
            Self::Iceberg(agent) => agent.think_for_one_tick(),
            Self::Kiwi(agent) => agent.think_for_one_tick(),
        }
    }

    fn evaluation(&self) -> Evaluation {
        match self {
            Self::Avocado(agent) => agent.evaluation(),
            Self::Cherry(agent) => agent.evaluation(),
            Self::Fajita(agent) => agent.evaluation(),
            Self::Garlic(agent) => agent.evaluation(),
            Self::Iceberg(agent) => agent.evaluation(),
            Self::Kiwi(agent) => agent.evaluation(),
        }
    }

    fn line(&self) -> Vec<Action> {
        let mut line = Vec::new();
        match self {
            Self::Avocado(agent) => agent.write_optimal_lop(&mut line),
            Self::Cherry(agent) => agent.write_optimal_lop(&mut line),
            Self::Fajita(agent) => agent.write_optimal_lop(&mut line),
            Self::Garlic(agent) => agent.write_optimal_lop(&mut line),
            Self::Iceberg(agent) => agent.write_optimal_lop(&mut line),
            Self::Kiwi(agent) => agent.write_optimal_lop(&mut line),
        }
        line
    }
}

#[derive(Clone)]
struct Config {
    pairs: u64,
    time_per_ply: Duration,
    seed_start: u64,
    max_plies: u32,
    save_results: SaveMode,
    output_root: PathBuf,
    matchups: Vec<Matchup>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SaveMode {
    Off,
    PerPly,
    PerGame,
}

impl SaveMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "per-ply" | "per_ply" | "perply" => Ok(Self::PerPly),
            "per-game" | "per_game" | "pergame" => Ok(Self::PerGame),
            _ => Err(format!(
                "invalid value `{value}` for `--save-results`; expected off, per-ply, or per-game"
            )),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::PerPly => "per-ply",
            Self::PerGame => "per-game",
        }
    }
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
}

#[derive(Clone, Copy)]
struct MatchupResult {
    first: AgentKind,
    second: AgentKind,
    first_wins: u64,
    second_wins: u64,
    draws: u64,
    elapsed: Duration,
}

#[derive(Clone, Copy, Default)]
struct Standing {
    wins: u64,
    losses: u64,
    draws: u64,
}

struct TournamentLogger {
    file: Mutex<Option<fs::File>>,
}

impl TournamentLogger {
    fn create(tournament_directory: Option<&Path>) -> Result<Arc<Self>, String> {
        let file = tournament_directory
            .map(|directory| {
                let path = directory.join("log.txt");
                fs::File::create(&path).map_err(|error| {
                    format!("cannot create tournament log {}: {error}", path.display())
                })
            })
            .transpose()?;
        Ok(Arc::new(Self {
            file: Mutex::new(file),
        }))
    }

    fn line(&self, message: impl AsRef<str>) -> Result<(), String> {
        let message = message.as_ref();
        let mut file = self
            .file
            .lock()
            .map_err(|_| "tournament log lock is poisoned".to_owned())?;
        if let Some(file) = file.as_mut() {
            writeln!(file, "{message}")
                .and_then(|()| file.flush())
                .map_err(|error| format!("cannot write tournament log: {error}"))?;
        }

        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        writeln!(stdout, "{message}")
            .and_then(|()| stdout.flush())
            .map_err(|error| format!("cannot write tournament output: {error}"))
    }
}

impl Standing {
    fn games(self) -> u64 {
        self.wins + self.losses + self.draws
    }

    fn points(self) -> f64 {
        self.wins as f64 + self.draws as f64 * 0.5
    }

    fn percentage(self) -> f64 {
        if self.games() == 0 {
            0.0
        } else {
            100.0 * self.points() / self.games() as f64
        }
    }
}

fn main() -> ExitCode {
    let result = match parse_args() {
        Ok(Some(config)) => run(config),
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("agent-arena: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args() -> Result<Option<Config>, String> {
    parse_args_from(env::args().skip(1))
}

fn parse_args_from<I, S>(args: I) -> Result<Option<Config>, String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut config = Config {
        pairs: DEFAULT_PAIRS,
        time_per_ply: Duration::from_millis(DEFAULT_MILLISECONDS),
        seed_start: 0,
        max_plies: DEFAULT_MAX_PLIES,
        save_results: SaveMode::PerPly,
        output_root: PathBuf::from(DEFAULT_OUTPUT_ROOT),
        matchups: Vec::new(),
    };
    let mut args = args.into_iter().map(Into::into);
    while let Some(flag) = args.next() {
        if flag == "--help" || flag == "-h" {
            println!("{}", usage());
            return Ok(None);
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for `{flag}`\n{}", usage()))?;
        match flag.as_str() {
            "--pairs" => config.pairs = parse(&flag, &value)?,
            "--milliseconds" => config.time_per_ply = Duration::from_millis(parse(&flag, &value)?),
            "--seed-start" => config.seed_start = parse(&flag, &value)?,
            "--max-plies" => config.max_plies = parse(&flag, &value)?,
            "--save-results" => config.save_results = SaveMode::parse(&value)?,
            "--output-root" => config.output_root = PathBuf::from(value),
            "--matchup" => {
                let matchup = parse_matchup(&value)?;
                if config.matchups.contains(&matchup) {
                    return Err(format!(
                        "duplicate matchup `{}-vs-{}`",
                        matchup.0.slug(),
                        matchup.1.slug()
                    ));
                }
                config.matchups.push(matchup);
            }
            _ => return Err(format!("unknown option `{flag}`\n{}", usage())),
        }
    }
    if config.pairs == 0 {
        return Err("`--pairs` must be positive".to_owned());
    }
    if config.time_per_ply.is_zero() {
        return Err("`--milliseconds` must be positive".to_owned());
    }
    if config.max_plies == 0 {
        return Err("`--max-plies` must be positive".to_owned());
    }
    if config.matchups.is_empty() {
        config.matchups = default_matchups();
    }
    Ok(Some(config))
}

fn parse_matchup(value: &str) -> Result<Matchup, String> {
    let normalized = value.to_ascii_lowercase();
    let (first, second) = normalized.split_once("-vs-").ok_or_else(|| {
        format!(
            "invalid matchup `{value}`; expected AGENT-vs-AGENT (for example, avocado-vs-garlic)"
        )
    })?;
    let mut first = AgentKind::parse(first)?;
    let mut second = AgentKind::parse(second)?;
    if first == second {
        return Err(format!(
            "invalid matchup `{value}`; an agent cannot play itself"
        ));
    }
    if second < first {
        std::mem::swap(&mut first, &mut second);
    }
    Ok((first, second))
}

fn default_matchups() -> Vec<Matchup> {
    let mut matchups = Vec::new();
    for (index, &first) in DEFAULT_AGENTS.iter().enumerate() {
        for &second in &DEFAULT_AGENTS[index + 1..] {
            matchups.push((first, second));
        }
    }
    matchups
}

fn parse<T: std::str::FromStr>(flag: &str, value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid value `{value}` for `{flag}`"))
}

fn usage() -> &'static str {
    "usage: agent-arena [--pairs 10] [--milliseconds 10000] \
     [--seed-start 0] [--max-plies 256] \
     [--save-results off|per-ply|per-game] [--output-root agent-arena-results] \
     [--matchup AGENT-vs-AGENT]...\n\
     agents: avocado, cherry, fajita, garlic, iceberg, kiwi\n\
     omit --matchup to run the default round robin; repeat it to select multiple matchups"
}

fn run(config: Config) -> Result<(), String> {
    let tournament_directory = create_tournament_directory(&config)?;
    let logger = TournamentLogger::create(tournament_directory.as_deref())?;
    let matchup_labels = config
        .matchups
        .iter()
        .map(|(first, second)| format!("{}–{}", first.label(), second.label()))
        .collect::<Vec<_>>()
        .join(", ");
    logger.line(format!(
        "matchups: {matchup_labels}; {} paired seeds; {:.3}s/ply",
        config.pairs,
        config.time_per_ply.as_secs_f64(),
    ))?;
    logger.line(format!(
        "seeds: {} through {}; each matchup swaps sides on every seed",
        config.seed_start,
        config.seed_start.wrapping_add(config.pairs - 1),
    ))?;
    match &tournament_directory {
        Some(path) => logger.line(format!(
            "result saving: {}; tournament directory: {}",
            config.save_results.label(),
            path.display()
        ))?,
        None => logger.line("result saving: off")?,
    }
    let started = Instant::now();
    let mut handles = Vec::with_capacity(config.matchups.len());
    for &(first, second) in &config.matchups {
        let matchup_directory = tournament_directory
            .as_ref()
            .map(|directory| create_matchup_directory(directory, first, second))
            .transpose()?;
        let worker_config = config.clone();
        let worker_logger = Arc::clone(&logger);
        handles.push(thread::spawn(move || {
            run_matchup(
                worker_config,
                first,
                second,
                matchup_directory,
                worker_logger,
            )
        }));
    }
    let mut results = Vec::with_capacity(config.matchups.len());
    for handle in handles {
        let result = handle
            .join()
            .map_err(|_| "a matchup worker panicked".to_owned())??;
        results.push(result);
    }
    results.sort_by_key(|result| (result.first, result.second));
    print_summary(&results, started.elapsed(), &logger)?;
    Ok(())
}

fn run_matchup(
    config: Config,
    first: AgentKind,
    second: AgentKind,
    matchup_directory: Option<PathBuf>,
    logger: Arc<TournamentLogger>,
) -> Result<MatchupResult, String> {
    let started = Instant::now();
    let mut result = MatchupResult {
        first,
        second,
        first_wins: 0,
        second_wins: 0,
        draws: 0,
        elapsed: Duration::ZERO,
    };

    for pair in 0..config.pairs {
        let seed = config.seed_start.wrapping_add(pair);
        for first_side in [Player::Alpha, Player::Beta] {
            let alpha = if first_side == Player::Alpha {
                first
            } else {
                second
            };
            let beta = if first_side == Player::Beta {
                first
            } else {
                second
            };
            let game_started = Instant::now();
            let history_path = matchup_directory
                .as_ref()
                .map(|directory| directory.join(game_filename(seed, first, first_side)));
            let report = play_game(
                seed,
                alpha,
                beta,
                config.time_per_ply,
                config.max_plies,
                config.save_results,
                history_path,
            )
            .map_err(|error| {
                format!(
                    "{}–{} seed {seed}, {first:?}: {error}",
                    first.label(),
                    second.label()
                )
            })?;
            match report.outcome {
                Outcome::Winner(winner) if winner == first_side => result.first_wins += 1,
                Outcome::Winner(_) => result.second_wins += 1,
                Outcome::Draw => result.draws += 1,
            }
            logger.line(format_game_result(
                first,
                second,
                seed,
                first_side,
                &report,
                game_started.elapsed(),
            ))?;
        }
    }
    result.elapsed = started.elapsed();
    Ok(result)
}

fn format_game_result(
    first: AgentKind,
    second: AgentKind,
    seed: u64,
    first_side: Player,
    report: &GameReport,
    elapsed: Duration,
) -> String {
    let (alpha, beta) = if first_side == Player::Alpha {
        (first, second)
    } else {
        (second, first)
    };
    format!(
        "{}–{} seed {seed:>4}, {} as {first_side:?} -- {} in {} plies \
         (ticks alpha={}, beta={}, game={:.1}s)",
        first.label(),
        second.label(),
        first.label(),
        format_outcome(report.outcome, alpha, beta),
        report.plies,
        report.alpha_ticks,
        report.beta_ticks,
        elapsed.as_secs_f64(),
    )
}

fn format_outcome(outcome: Outcome, alpha: AgentKind, beta: AgentKind) -> String {
    match outcome {
        Outcome::Winner(Player::Alpha) => format!("Winner: {}(Alpha)", alpha.label()),
        Outcome::Winner(Player::Beta) => format!("Winner: {}(Beta)", beta.label()),
        Outcome::Draw => "Draw".to_owned(),
    }
}

fn format_matchup_result(result: &MatchupResult) -> String {
    format!(
        "{}–{} result: {}–{}–{} ({} wins, {} wins, draws) in {:.1}s",
        result.first.label(),
        result.second.label(),
        result.first_wins,
        result.second_wins,
        result.draws,
        result.first.label(),
        result.second.label(),
        result.elapsed.as_secs_f64(),
    )
}

fn play_game(
    seed: u64,
    alpha_kind: AgentKind,
    beta_kind: AgentKind,
    time_per_ply: Duration,
    max_plies: u32,
    save_mode: SaveMode,
    history_path: Option<PathBuf>,
) -> Result<GameReport, String> {
    let mut state = initial_state(seed);
    let mut alpha = alpha_kind.create();
    let mut beta = beta_kind.create();
    let mut alpha_ticks = 0u64;
    let mut beta_ticks = 0u64;
    let mut history = GameHistory::start(
        save_mode,
        history_path,
        &state,
        seed,
        alpha_kind,
        beta_kind,
        time_per_ply,
    )?;

    for ply in 0..max_plies {
        if let Some(winner) = state.winner() {
            history.complete()?;
            return Ok(GameReport {
                outcome: Outcome::Winner(winner),
                plies: ply,
                alpha_ticks,
                beta_ticks,
            });
        }
        let player = state.active_player;
        let (agent, ticks) = match player {
            Player::Alpha => (&mut alpha, &mut alpha_ticks),
            Player::Beta => (&mut beta, &mut beta_ticks),
        };
        agent.set_state(state.clone());
        let started = Instant::now();
        loop {
            agent.think_for_one_tick();
            *ticks += 1;
            if started.elapsed() >= time_per_ply {
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

        let turn_start = state.clone();
        let mut played = Vec::with_capacity(2);
        let mut completed = false;
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
        if !completed {
            return Err(format!(
                "{} returned an incomplete animal turn",
                match player {
                    Player::Alpha => alpha_kind.label(),
                    Player::Beta => beta_kind.label(),
                }
            ));
        }
        history.record_turn(ply + 1, &turn_start, &played)?;
        if let Some(winner) = state.winner() {
            history.complete()?;
            return Ok(GameReport {
                outcome: Outcome::Winner(winner),
                plies: ply + 1,
                alpha_ticks,
                beta_ticks,
            });
        }
        history.checkpoint()?;
    }

    history.complete()?;
    Ok(GameReport {
        outcome: Outcome::Draw,
        plies: max_plies,
        alpha_ticks,
        beta_ticks,
    })
}

struct GameHistory {
    mode: SaveMode,
    path: Option<PathBuf>,
    recorder: Option<HistoryRecorder>,
}

impl GameHistory {
    fn start(
        mode: SaveMode,
        path: Option<PathBuf>,
        state: &State,
        seed: u64,
        alpha: AgentKind,
        beta: AgentKind,
        time_per_ply: Duration,
    ) -> Result<Self, String> {
        let recorder = (mode != SaveMode::Off).then(|| {
            HistoryRecorder::new(
                state,
                seed,
                alpha.label(),
                beta.label(),
                time_per_ply,
                time_per_ply,
            )
        });
        let history = Self {
            mode,
            path,
            recorder,
        };
        if mode == SaveMode::PerPly {
            history.write(true)?;
        }
        Ok(history)
    }

    fn record_turn(
        &mut self,
        timeline_index: u32,
        state: &State,
        actions: &[Action],
    ) -> Result<(), String> {
        if let Some(recorder) = &mut self.recorder {
            recorder.record_turn(timeline_index, state, actions)?;
        }
        Ok(())
    }

    fn checkpoint(&self) -> Result<(), String> {
        if self.mode == SaveMode::PerPly {
            self.write(true)?;
        }
        Ok(())
    }

    fn complete(&self) -> Result<(), String> {
        if self.mode != SaveMode::Off {
            self.write(false)?;
        }
        Ok(())
    }

    fn write(&self, incomplete: bool) -> Result<(), String> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| "history path is missing while result saving is enabled".to_owned())?;
        let recorder = self.recorder.as_ref().ok_or_else(|| {
            "history recorder is missing while result saving is enabled".to_owned()
        })?;
        let temporary = path.with_extension("shgh.tmp");
        fs::write(&temporary, recorder.render(incomplete))
            .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
        fs::rename(&temporary, path).map_err(|error| {
            format!(
                "cannot replace {} with {}: {error}",
                path.display(),
                temporary.display()
            )
        })
    }
}

fn create_tournament_directory(config: &Config) -> Result<Option<PathBuf>, String> {
    if config.save_results == SaveMode::Off {
        return Ok(None);
    }
    fs::create_dir_all(&config.output_root).map_err(|error| {
        format!(
            "cannot create output root {}: {error}",
            config.output_root.display()
        )
    })?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock is before the Unix epoch: {error}"))?;
    let base = format!(
        "tournament-{}-{:03}-{}",
        now.as_secs(),
        now.subsec_millis(),
        std::process::id(),
    );
    for suffix in 0..1_000 {
        let name = if suffix == 0 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let path = config.output_root.join(name);
        match fs::create_dir(&path) {
            Ok(()) => return Ok(Some(path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(format!(
                    "cannot create tournament directory {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err(format!(
        "cannot allocate a unique tournament directory in {}",
        config.output_root.display()
    ))
}

fn create_matchup_directory(
    tournament_directory: &Path,
    first: AgentKind,
    second: AgentKind,
) -> Result<PathBuf, String> {
    let path = tournament_directory.join(format!("{}-vs-{}", first.slug(), second.slug()));
    fs::create_dir(&path).map_err(|error| {
        format!(
            "cannot create matchup directory {}: {error}",
            path.display()
        )
    })?;
    Ok(path)
}

fn game_filename(seed: u64, first: AgentKind, first_side: Player) -> String {
    format!(
        "seed-{seed:020}-{}-as-{}.shgh",
        first.slug(),
        player_slug(first_side),
    )
}

const fn player_slug(player: Player) -> &'static str {
    match player {
        Player::Alpha => "alpha",
        Player::Beta => "beta",
    }
}

fn print_summary(
    results: &[MatchupResult],
    elapsed: Duration,
    logger: &TournamentLogger,
) -> Result<(), String> {
    let mut standings = BTreeMap::new();
    for result in results {
        standings.entry(result.first).or_insert(Standing::default());
        standings
            .entry(result.second)
            .or_insert(Standing::default());
        let first = standings
            .get_mut(&result.first)
            .expect("all agents have standings");
        first.wins += result.first_wins;
        first.losses += result.second_wins;
        first.draws += result.draws;
        let second = standings
            .get_mut(&result.second)
            .expect("all agents have standings");
        second.wins += result.second_wins;
        second.losses += result.first_wins;
        second.draws += result.draws;
    }

    let mut ranked = standings.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(left_agent, left), (right_agent, right)| {
        right
            .points()
            .total_cmp(&left.points())
            .then_with(|| right.wins.cmp(&left.wins))
            .then_with(|| left_agent.cmp(right_agent))
    });

    logger.line("")?;
    for result in results {
        logger.line(format_matchup_result(result))?;
    }
    logger.line("")?;
    logger.line("FINAL STANDINGS")?;
    logger.line("rank  agent       W   L   D   points   score")?;
    for (index, (agent, standing)) in ranked.iter().enumerate() {
        logger.line(format!(
            "{:>4}  {:<9} {:>3} {:>3} {:>3}   {:>5.1}   {:>5.1}%",
            index + 1,
            agent.label(),
            standing.wins,
            standing.losses,
            standing.draws,
            standing.points(),
            standing.percentage(),
        ))?;
    }
    logger.line(format!("elapsed: {:.1}s", elapsed.as_secs_f64()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_mode_values_are_cli_friendly() {
        assert_eq!(SaveMode::parse("off"), Ok(SaveMode::Off));
        assert_eq!(SaveMode::parse("Per-Ply"), Ok(SaveMode::PerPly));
        assert_eq!(SaveMode::parse("per_game"), Ok(SaveMode::PerGame));
        assert!(SaveMode::parse("sometimes").is_err());
    }

    #[test]
    fn matchup_values_are_case_insensitive_and_canonicalized() {
        assert_eq!(
            parse_matchup("Garlic-vs-Avocado"),
            Ok((AgentKind::Avocado, AgentKind::Garlic))
        );
        assert_eq!(
            parse_matchup("iceberg-vs-avocado"),
            Ok((AgentKind::Avocado, AgentKind::Iceberg))
        );
        assert!(parse_matchup("avocado-vs-avocado").is_err());
        assert!(parse_matchup("avocado-garlic").is_err());
        assert!(parse_matchup("avocado-vs-potato").is_err());
    }

    #[test]
    fn omitted_matchups_use_the_default_round_robin() {
        let config = parse_args_from(Vec::<String>::new())
            .unwrap()
            .expect("default arguments should run the arena");
        assert_eq!(
            DEFAULT_AGENTS,
            [
                AgentKind::Cherry,
                AgentKind::Garlic,
                AgentKind::Fajita,
                AgentKind::Kiwi,
            ]
        );
        assert_eq!(config.matchups, default_matchups());
        assert_eq!(config.matchups.len(), 6);
        assert!(config.matchups.iter().all(|(first, second)| {
            !matches!(first, AgentKind::Avocado | AgentKind::Iceberg)
                && !matches!(second, AgentKind::Avocado | AgentKind::Iceberg)
        }));
    }

    #[test]
    fn game_result_names_the_winning_agent_and_side() {
        let report = GameReport {
            outcome: Outcome::Winner(Player::Beta),
            plies: 21,
            alpha_ticks: 811_950_793,
            beta_ticks: 16_229_932,
        };
        assert_eq!(
            format_game_result(
                AgentKind::Avocado,
                AgentKind::Fajita,
                9,
                Player::Alpha,
                &report,
                Duration::from_millis(213_100),
            ),
            "Avocado–Fajita seed    9, Avocado as Alpha -- Winner: Fajita(Beta) in 21 plies (ticks alpha=811950793, beta=16229932, game=213.1s)"
        );
    }

    #[test]
    fn renamed_save_results_flag_is_accepted() {
        let config = parse_args_from(["--save-results", "off"])
            .unwrap()
            .expect("save-results should be accepted");
        assert_eq!(config.save_results, SaveMode::Off);

        let error = parse_args_from(["--save-games", "off"])
            .err()
            .expect("the old save-games spelling should be rejected");
        assert!(error.contains("unknown option `--save-games`"));
    }

    #[test]
    fn repeated_matchup_flags_select_only_those_matches() {
        let config = parse_args_from([
            "--matchup",
            "garlic-vs-avocado",
            "--matchup",
            "cherry-vs-fajita",
        ])
        .unwrap()
        .expect("selected matchups should run the arena");
        assert_eq!(
            config.matchups,
            vec![
                (AgentKind::Avocado, AgentKind::Garlic),
                (AgentKind::Cherry, AgentKind::Fajita),
            ]
        );
    }

    #[test]
    fn duplicate_matchup_flags_are_rejected_regardless_of_order() {
        let error = parse_args_from([
            "--matchup",
            "avocado-vs-garlic",
            "--matchup",
            "garlic-vs-avocado",
        ])
        .err()
        .expect("duplicate matchup should fail");
        assert!(error.contains("duplicate matchup `avocado-vs-garlic`"));
    }

    #[test]
    fn game_filenames_distinguish_the_side_swap() {
        assert_eq!(
            game_filename(42, AgentKind::Avocado, Player::Alpha),
            "seed-00000000000000000042-avocado-as-alpha.shgh"
        );
        assert_eq!(
            game_filename(42, AgentKind::Avocado, Player::Beta),
            "seed-00000000000000000042-avocado-as-beta.shgh"
        );
    }
}
