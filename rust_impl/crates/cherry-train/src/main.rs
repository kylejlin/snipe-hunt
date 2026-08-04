use agent_cherry::{
    ACTION_SIZE, INPUT_SIZE, Model, Search, action_index, encode_state, state_key,
    training::{Adam, Sample},
};
use snipe_core::{Action, Player};
use snipe_prng::initial_state;
use std::{
    collections::HashSet,
    env, fs,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::{self, ExitCode},
    sync::{
        atomic::{AtomicU8, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEFAULT_RUN_DIR: &str = "training/cherry-main";
const DEFAULT_HOURS: f64 = 8.0;
const DEFAULT_SIMULATIONS: usize = 512;
const MAX_REPLAY: usize = 500_000;
const MAX_ATOMIC_ACTIONS: usize = 256;
const BATCH_SIZE: usize = 32;
const BATCHES_PER_GAME: usize = 6;
const REPLAY_SAVE_INTERVAL: u64 = 25;
const PROGRESS_REPORT_INTERVAL: u64 = 500;
const PROMOTION_INTERVAL: u64 = 1_000;
const PROMOTION_PAIRS: usize = 96;
const DIRICHLET_ALPHA: f32 = 0.3;
const ROOT_NOISE_FRACTION: f32 = 0.25;
const PACKED_INPUT_SIZE: usize = INPUT_SIZE.div_ceil(4);
const REPLAY_MAGIC_V1: &[u8; 8] = b"CHREPLAY";
const REPLAY_MAGIC_V2: &[u8; 8] = b"CHREPL02";
static SHUTDOWN_REQUESTS: AtomicU8 = AtomicU8::new(0);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cherry-train: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let command = arguments.first().map(String::as_str).unwrap_or("train");
    let run_dir = option(&arguments, "--run-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_DIR));
    match command {
        "train" => {
            let hours = option(&arguments, "--hours")
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_HOURS);
            let simulations = option(&arguments, "--simulations")
                .and_then(|value| value.parse().ok())
                .unwrap_or(DEFAULT_SIMULATIONS);
            let workers = option(&arguments, "--workers")
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(default_workers);
            let progress_reports = parse_on_off(&arguments, "--progress-reports", true)?;
            train(
                &run_dir,
                Duration::from_secs_f64((hours * 3600.0).max(1.0)),
                simulations,
                workers,
                progress_reports,
            )
        }
        "publish" => publish(
            &run_dir,
            arguments
                .iter()
                .any(|argument| argument == "--allow-when-dirty"),
        ),
        "status" => status(&run_dir),
        "evaluate" => evaluate_command(&run_dir, &arguments),
        "audit" => audit_command(&run_dir, &arguments),
        "help" | "--help" | "-h" => {
            help();
            Ok(())
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command {other:?}; use `cherry-train help`"),
        )),
    }
}

fn option<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn parse_on_off(arguments: &[String], name: &str, default: bool) -> io::Result<bool> {
    let Some(index) = arguments.iter().position(|argument| argument == name) else {
        return Ok(default);
    };
    let value = arguments.get(index + 1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} requires on or off"),
        )
    })?;
    match value.to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid value {value:?} for {name}; expected on or off"),
        )),
    }
}

fn help() {
    println!(
        "Cherry self-play trainer\n\
         \n\
         train          [--run-dir PATH] [--hours N] [--simulations N] [--workers N]\n\
                        [--progress-reports on|off]\n\
         status        [--run-dir PATH]\n\
         evaluate      [--run-dir PATH] [--simulations N]\n\
         audit         [--run-dir PATH] [--simulations N] [--pairs N]\n\
         publish       [--run-dir PATH] [--allow-when-dirty]\n\
         \n\
         Publishing requires a clean Git worktree unless --allow-when-dirty is present.\n\
         Publishing advances the browser's minor version and resets its patch to zero.\n\
         Simulations is a base; wide positions automatically receive at least 3x legal actions.\n\
         Weights and optimizer checkpoint after every completed game; compact replay every 25.\n\
         Timestamped progress reports are on by default and print every 500 games.\n\
         Ctrl+C requests a graceful stop after the current self-play batch or arena and\n\
         writes a full checkpoint. Press Ctrl+C again only to force an immediate exit."
    );
}

struct RunState {
    model: Model,
    champion: Model,
    optimizer: Adam,
    replay: ReplayBuffer,
    games: u64,
    promotions: u64,
    rng: Rng,
}

struct CompactSample {
    input: [u8; PACKED_INPUT_SIZE],
    policy: Box<[(u16, u16)]>,
    value: i8,
}

struct ReplayBuffer {
    entries: Vec<CompactSample>,
    next: usize,
    capacity: usize,
}

impl ReplayBuffer {
    fn new() -> Self {
        Self::with_capacity(MAX_REPLAY)
    }

    fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            entries: Vec::with_capacity(capacity),
            next: 0,
            capacity,
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn push(&mut self, sample: CompactSample) {
        if self.entries.len() < self.capacity {
            self.entries.push(sample);
        } else {
            self.entries[self.next] = sample;
            self.next = (self.next + 1) % self.capacity;
        }
    }

    fn extend(&mut self, samples: impl IntoIterator<Item = CompactSample>) {
        for sample in samples {
            self.push(sample);
        }
    }

    fn get(&self, index: usize) -> &CompactSample {
        &self.entries[index]
    }

    fn chronological(&self) -> impl Iterator<Item = &CompactSample> {
        let split = if self.entries.len() == self.capacity {
            self.next
        } else {
            0
        };
        self.entries[split..]
            .iter()
            .chain(self.entries[..split].iter())
    }
}

fn default_workers() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .saturating_sub(1)
        .max(1)
}

#[derive(Clone, Copy)]
struct ProgressSnapshot {
    games: u64,
    candidate_steps: u64,
    champion_steps: u64,
    promotions: u64,
    replay_positions: usize,
}

impl ProgressSnapshot {
    fn from_run(run: &RunState) -> Self {
        Self {
            games: run.games,
            candidate_steps: run.model.training_steps,
            champion_steps: run.champion.training_steps,
            promotions: run.promotions,
            replay_positions: run.replay.len(),
        }
    }
}

#[derive(Clone, Copy)]
enum ArenaDecision {
    Promoted,
    LeagueGuardRejected,
    Continued,
}

fn print_progress(enabled: bool, message: String) -> io::Result<()> {
    if enabled {
        let mut output = io::stdout().lock();
        writeln!(
            output,
            "{}",
            timestamped_report(SystemTime::now(), &message)
        )?;
        output.flush()?;
    }
    Ok(())
}

fn timestamped_report(time: SystemTime, message: &str) -> String {
    format!("[{}] {message}", utc_timestamp(time))
}

fn format_start_report(
    snapshot: ProgressSnapshot,
    simulations: usize,
    workers: usize,
    run_dir: &Path,
) -> String {
    format!(
        "Cherry training resumed at game {}, candidate step {}, with champion step {}. Totals: {} promotions and {} replay positions. Base simulations/action: {}; workers: {}; run directory: {}.",
        format_count(snapshot.games),
        format_count(snapshot.candidate_steps),
        format_count(snapshot.champion_steps),
        format_count(snapshot.promotions),
        format_count(snapshot.replay_positions as u64),
        format_count(simulations as u64),
        format_count(workers as u64),
        run_dir.display(),
    )
}

fn format_periodic_report(
    snapshot: ProgressSnapshot,
    last_loss: f32,
    winner: Option<Player>,
    actions: usize,
) -> String {
    let next_arena = snapshot.games.div_ceil(PROMOTION_INTERVAL) * PROMOTION_INTERVAL;
    let next_arena = if next_arena == snapshot.games {
        next_arena + PROMOTION_INTERVAL
    } else {
        next_arena
    };
    let latest_game = winner.map_or_else(
        || format!("The latest self-play game was drawn after {actions} actions."),
        |winner| {
            format!(
                "{} won the latest self-play game after {actions} actions.",
                if winner == Player::Alpha {
                    "Alpha"
                } else {
                    "Beta"
                }
            )
        },
    );
    format!(
        "Cherry is training normally through game {}. Candidate step {}; champion step {}; {} promotions; replay contains {} positions; latest loss {:.4}. {latest_game} The next arena is at game {}.",
        format_count(snapshot.games),
        format_count(snapshot.candidate_steps),
        format_count(snapshot.champion_steps),
        format_count(snapshot.promotions),
        format_count(snapshot.replay_positions as u64),
        last_loss,
        format_count(next_arena),
    )
}

fn format_arena_report(
    snapshot: ProgressSnapshot,
    candidate_steps: u64,
    incumbent_steps: u64,
    result: ArenaResult,
    decision: ArenaDecision,
) -> String {
    let score = result.score * 100.0;
    let lower_99 = result.lower_bound * 100.0;
    match decision {
        ArenaDecision::Promoted => format!(
            "Promotion {} at game {}: candidate step {} scored {:.1}% ({:.1}% lower 99% confidence bound), passed the validated league guard, and became the new champion. Training continues with {} replay positions.",
            format_count(snapshot.promotions),
            format_count(snapshot.games),
            format_count(candidate_steps),
            score,
            lower_99,
            format_count(snapshot.replay_positions as u64),
        ),
        ArenaDecision::LeagueGuardRejected => format!(
            "At game {}, candidate step {} scored {:.1}% ({:.1}% lower 99% confidence bound) against champion step {}, but did not pass the validated league guard. No promotion occurred; training continues with {} promotions and {} replay positions.",
            format_count(snapshot.games),
            format_count(candidate_steps),
            score,
            lower_99,
            format_count(incumbent_steps),
            format_count(snapshot.promotions),
            format_count(snapshot.replay_positions as u64),
        ),
        ArenaDecision::Continued => format!(
            "At game {}, candidate step {} scored {:.1}% ({:.1}% lower 99% confidence bound) against champion step {}. No promotion occurred; training continues with {} promotions and {} replay positions.",
            format_count(snapshot.games),
            format_count(candidate_steps),
            score,
            lower_99,
            format_count(incumbent_steps),
            format_count(snapshot.promotions),
            format_count(snapshot.replay_positions as u64),
        ),
    }
}

fn format_completion_report(snapshot: ProgressSnapshot) -> String {
    format!(
        "Training window complete at game {}. Candidate step {}; champion step {}; {} promotions; replay contains {} positions.",
        format_count(snapshot.games),
        format_count(snapshot.candidate_steps),
        format_count(snapshot.champion_steps),
        format_count(snapshot.promotions),
        format_count(snapshot.replay_positions as u64),
    )
}

fn format_shutdown_report(snapshot: ProgressSnapshot, run_dir: &Path) -> String {
    format!(
        "Graceful shutdown complete at game {}. Candidate step {}; champion step {}; {} promotions; replay contains {} positions. A full resumable checkpoint was saved in {}.",
        format_count(snapshot.games),
        format_count(snapshot.candidate_steps),
        format_count(snapshot.champion_steps),
        format_count(snapshot.promotions),
        format_count(snapshot.replay_positions as u64),
        run_dir.display(),
    )
}

fn format_count(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, chunk) in digits.as_bytes().rchunks(3).rev().enumerate() {
        if index > 0 {
            formatted.push(',');
        }
        formatted.push_str(std::str::from_utf8(chunk).expect("decimal digits are valid UTF-8"));
    }
    formatted
}

fn utc_timestamp(time: SystemTime) -> String {
    let seconds = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    let second = day_seconds % 60;
    let (year, month, day) = civil_date_from_unix_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn train(
    run_dir: &Path,
    duration: Duration,
    simulations: usize,
    workers: usize,
    progress_reports: bool,
) -> io::Result<()> {
    if simulations == 0 || workers == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "simulations and workers must both be positive",
        ));
    }
    fs::create_dir_all(run_dir)?;
    install_shutdown_handler()?;
    let mut run = load_run(run_dir, true)?;
    let started = Instant::now();
    let mut training_batch = Vec::with_capacity(BATCH_SIZE);
    let mut last_loss = 0.0;
    let mut last_arena = None;
    print_progress(
        progress_reports,
        format_start_report(
            ProgressSnapshot::from_run(&run),
            simulations,
            workers,
            &absolute(run_dir)?,
        ),
    )?;

    while started.elapsed() < duration && !shutdown_requested() {
        let model = run.model.clone();
        let jobs = (0..workers.max(1))
            .map(|_| (run.rng.next_u64(), run.rng.next_u64()))
            .collect::<Vec<_>>();
        let (sender, receiver) = mpsc::channel();
        thread::scope(|scope| {
            for (seed, rng_seed) in jobs {
                let sender = sender.clone();
                let model = &model;
                scope.spawn(move || {
                    let mut rng = Rng::new(rng_seed);
                    let result = self_play(model, seed, simulations, &mut rng);
                    sender.send(result).expect("trainer receiver remains alive");
                });
            }
            drop(sender);
            for (samples, winner, actions) in receiver {
                run.replay.extend(samples);
                run.games += 1;

                let mut loss = 0.0;
                if !run.replay.is_empty() {
                    for _ in 0..BATCHES_PER_GAME {
                        fill_random_batch(
                            &run.replay,
                            BATCH_SIZE,
                            &mut run.rng,
                            &mut training_batch,
                        );
                        loss += run
                            .model
                            .train_batch(&training_batch, &mut run.optimizer, 0.0005);
                    }
                    loss /= BATCHES_PER_GAME as f32;
                }
                last_loss = loss;

                let mut arena_result = None;
                let mut arena_report = None;
                if run.games % PROMOTION_INTERVAL == 0 {
                    let candidate_steps = run.model.training_steps;
                    let incumbent_steps = run.champion.training_steps;
                    let result = arena(
                        &run.model,
                        &run.champion,
                        simulations,
                        run.games,
                        PROMOTION_PAIRS,
                        workers,
                    );
                    let mut decision = ArenaDecision::Continued;
                    if result.lower_bound > 0.5 {
                        if passes_league_guard(
                            run_dir,
                            &run.model,
                            &run.champion,
                            simulations.max(8),
                            run.games,
                            workers,
                        )? {
                            run.champion = run.model.clone();
                            run.promotions += 1;
                            archive_champion(run_dir, &run)?;
                            decision = ArenaDecision::Promoted;
                        } else {
                            decision = ArenaDecision::LeagueGuardRejected;
                        }
                    }
                    append_arena_report(run_dir, &run, result)?;
                    arena_result = Some(result);
                    arena_report = Some(format_arena_report(
                        ProgressSnapshot::from_run(&run),
                        candidate_steps,
                        incumbent_steps,
                        result,
                        decision,
                    ));
                }
                last_arena = arena_result;
                save_run(
                    run_dir,
                    &run,
                    loss,
                    arena_result,
                    run.games < 10 || run.games % REPLAY_SAVE_INTERVAL == 0,
                )?;
                if let Some(report) = arena_report {
                    print_progress(progress_reports, report)?;
                } else if run.games % PROGRESS_REPORT_INTERVAL == 0 {
                    print_progress(
                        progress_reports,
                        format_periodic_report(
                            ProgressSnapshot::from_run(&run),
                            loss,
                            winner,
                            actions,
                        ),
                    )?;
                }
            }
            Ok::<(), io::Error>(())
        })?;
    }
    save_run(run_dir, &run, last_loss, last_arena, true)?;
    let snapshot = ProgressSnapshot::from_run(&run);
    if shutdown_requested() {
        print_progress(true, format_shutdown_report(snapshot, &absolute(run_dir)?))?;
    } else {
        print_progress(progress_reports, format_completion_report(snapshot))?;
    }
    Ok(())
}

fn install_shutdown_handler() -> io::Result<()> {
    SHUTDOWN_REQUESTS.store(0, Ordering::SeqCst);
    ctrlc::set_handler(|| {
        if SHUTDOWN_REQUESTS.fetch_add(1, Ordering::SeqCst) == 0 {
            eprintln!(
                "\nGraceful shutdown requested. Cherry will finish the current self-play batch \
                 or arena, then save a full checkpoint. Press Ctrl+C again to force an immediate \
                 exit without saving."
            );
        } else {
            eprintln!("\nSecond interrupt received; exiting immediately without saving.");
            process::exit(130);
        }
    })
    .map_err(|error| io::Error::other(format!("could not install Ctrl+C handler: {error}")))
}

fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTS.load(Ordering::SeqCst) != 0
}

fn self_play(
    model: &Model,
    seed: u64,
    simulations: usize,
    rng: &mut Rng,
) -> (Vec<CompactSample>, Option<Player>, usize) {
    struct PendingSample {
        input: [u8; PACKED_INPUT_SIZE],
        policy: Box<[(u16, u16)]>,
        player: Player,
    }

    let mut state = initial_state(seed);
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    let mut winner = None;
    let mut search = Search::new(state.clone(), model);
    for action_number in 0..MAX_ATOMIC_ACTIONS {
        if let Some(found) = state.winner() {
            winner = Some(found);
            break;
        }
        if !seen.insert(state_key(&state)) {
            break;
        }
        search.add_root_dirichlet_noise(DIRICHLET_ALPHA, ROOT_NOISE_FRACTION, rng.next_u64());
        search.simulate_n(
            model,
            adaptive_simulations(simulations, search.root_action_count()),
        );
        let policy = search.policy(if action_number < 30 { 1.0 } else { 0.05 });
        if policy.is_empty() {
            winner = state.winner();
            break;
        }
        let sparse_policy = policy
            .iter()
            .filter(|(_, probability)| *probability > 0.0)
            .filter_map(|&(action, probability)| {
                let weight = (probability * f32::from(u16::MAX)).round() as u16;
                (weight > 0).then_some((
                    u16::try_from(action_index(&state, action)).expect("action index fits u16"),
                    weight,
                ))
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        records.push(PendingSample {
            input: compact_input(&encode_state(&state)),
            policy: sparse_policy,
            player: state.active_player,
        });
        let action = sample_policy(&policy, rng);
        state = state
            .apply(action)
            .expect("MCTS policy contains legal actions");
        if !search.advance(action, model) {
            search = Search::new(state.clone(), model);
        }
    }
    winner = winner.or_else(|| state.winner());
    let samples = records
        .into_iter()
        .map(|record| CompactSample {
            input: record.input,
            policy: record.policy,
            value: winner.map_or(0, |won| if won == record.player { 1 } else { -1 }),
        })
        .collect::<Vec<_>>();
    (samples, winner, seen.len())
}

fn sample_policy(policy: &[(Action, f32)], rng: &mut Rng) -> Action {
    let target = rng.unit();
    let mut cumulative = 0.0;
    for &(action, probability) in policy {
        cumulative += probability;
        if target <= cumulative {
            return action;
        }
    }
    policy.last().expect("non-empty policy").0
}

fn adaptive_simulations(base: usize, branching_factor: usize) -> usize {
    base.max(branching_factor.saturating_mul(3).min(1_536))
}

fn compact_input(input: &[f32; INPUT_SIZE]) -> [u8; PACKED_INPUT_SIZE] {
    let mut packed = [0; PACKED_INPUT_SIZE];
    for (index, value) in input.iter().copied().enumerate() {
        let quantized = (value * 2.0).round();
        debug_assert!(
            (value - quantized * 0.5).abs() < f32::EPSILON,
            "state features must be exact half increments"
        );
        debug_assert!((0.0..=3.0).contains(&quantized));
        set_packed_feature(&mut packed, index, quantized as u8);
    }
    packed
}

fn set_packed_feature(packed: &mut [u8], index: usize, value: u8) {
    let shift = (index % 4) * 2;
    packed[index / 4] = (packed[index / 4] & !(0b11 << shift)) | (value << shift);
}

fn packed_feature(packed: &[u8], index: usize) -> u8 {
    (packed[index / 4] >> ((index % 4) * 2)) & 0b11
}

fn fill_random_batch(replay: &ReplayBuffer, size: usize, rng: &mut Rng, batch: &mut Vec<Sample>) {
    let target_len = size.min(replay.len());
    batch.resize_with(target_len, || Sample {
        input: [0.0; INPUT_SIZE],
        policy: [0.0; ACTION_SIZE],
        value: 0.0,
    });
    for output in batch {
        let sample = replay.get((rng.next_u64() as usize) % replay.len());
        for (index, destination) in output.input.iter_mut().enumerate() {
            *destination = f32::from(packed_feature(&sample.input[..], index)) * 0.5;
        }
        output.policy.fill(0.0);
        let total_weight = sample
            .policy
            .iter()
            .map(|(_, weight)| u32::from(*weight))
            .sum::<u32>()
            .max(1);
        for &(index, weight) in sample.policy.iter() {
            output.policy[usize::from(index)] = weight as f32 / total_weight as f32;
        }
        output.value = f32::from(sample.value);
    }
}

#[derive(Clone, Copy)]
struct ArenaResult {
    score: f32,
    lower_bound: f32,
    games: usize,
}

fn arena(
    candidate: &Model,
    incumbent: &Model,
    simulations: usize,
    round: u64,
    pairs: usize,
    workers: usize,
) -> ArenaResult {
    let worker_count = workers.max(1).min(pairs.max(1));
    let (sender, receiver) = mpsc::channel();
    thread::scope(|scope| {
        for worker in 0..worker_count {
            let sender = sender.clone();
            scope.spawn(move || {
                for pair in (worker..pairs).step_by(worker_count) {
                    let seed = 0xA2E1_0000_0000_0000 ^ round.rotate_left(17) ^ pair as u64;
                    let alpha_score = play_match(candidate, incumbent, seed, simulations);
                    let beta_score = 1.0 - play_match(incumbent, candidate, seed, simulations);
                    sender
                        .send((pair, (alpha_score + beta_score) * 0.5))
                        .expect("arena receiver remains alive");
                }
            });
        }
        drop(sender);
    });
    let mut paired_scores = vec![0.0; pairs];
    for (pair, score) in receiver {
        paired_scores[pair] = score;
    }
    let score = paired_scores.iter().sum::<f32>() / pairs.max(1) as f32;
    let variance = if pairs > 1 {
        paired_scores
            .iter()
            .map(|value| (value - score).powi(2))
            .sum::<f32>()
            / (pairs - 1) as f32
    } else {
        0.25
    };
    let standard_error = (variance / pairs.max(1) as f32).sqrt();
    ArenaResult {
        score,
        lower_bound: (score - 2.326 * standard_error).clamp(0.0, 1.0),
        games: pairs * 2,
    }
}

/// Returns Alpha's score, with a capped/repeated game worth one half.
fn play_match(alpha: &Model, beta: &Model, seed: u64, simulations: usize) -> f32 {
    play_match_with_budgets(alpha, simulations, beta, simulations, seed)
}

fn play_match_with_budgets(
    alpha: &Model,
    alpha_simulations: usize,
    beta: &Model,
    beta_simulations: usize,
    seed: u64,
) -> f32 {
    let mut state = initial_state(seed);
    let mut seen = HashSet::new();
    let mut search = None;
    for _ in 0..MAX_ATOMIC_ACTIONS {
        if let Some(winner) = state.winner() {
            return if winner == Player::Alpha { 1.0 } else { 0.0 };
        }
        if !seen.insert(state_key(&state)) {
            return 0.5;
        }
        let moving_player = state.active_player;
        let (model, simulations) = if moving_player == Player::Alpha {
            (alpha, alpha_simulations)
        } else {
            (beta, beta_simulations)
        };
        let current_search = search.get_or_insert_with(|| Search::new(state.clone(), model));
        current_search.simulate_n(
            model,
            adaptive_simulations(simulations, current_search.root_action_count()),
        );
        let Some((action, _)) = current_search
            .policy(0.0)
            .into_iter()
            .find(|(_, p)| *p > 0.0)
        else {
            return 0.5;
        };
        state = state.apply(action).expect("search returns legal action");
        if state.active_player != moving_player || !current_search.advance(action, model) {
            search = None;
        }
    }
    0.5
}

fn audit_command(run_dir: &Path, arguments: &[String]) -> io::Result<()> {
    let simulations = option(arguments, "--simulations")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SIMULATIONS);
    let pairs = option(arguments, "--pairs")
        .and_then(|value| value.parse().ok())
        .unwrap_or(64);
    let run = load_run(run_dir, false)?;
    let adversary_simulations = simulations.saturating_mul(4);
    let mut champion_scores = Vec::with_capacity(pairs);
    for pair in 0..pairs {
        let seed = 0xB35A_0000_0000_0000 ^ run.games.rotate_left(11) ^ pair as u64;
        let as_alpha = play_match_with_budgets(
            &run.model,
            simulations,
            &run.model,
            adversary_simulations,
            seed,
        );
        let as_beta = 1.0
            - play_match_with_budgets(
                &run.model,
                adversary_simulations,
                &run.model,
                simulations,
                seed,
            );
        champion_scores.push((as_alpha + as_beta) * 0.5);
    }
    let score = champion_scores.iter().sum::<f32>() / pairs.max(1) as f32;
    println!(
        "search exploitability audit: champion_score={score:.3}, estimated_exploitability={:.3}, games={}, champion_simulations={}, adversary_simulations={}",
        (0.5 - score).max(0.0),
        pairs * 2,
        simulations,
        adversary_simulations,
    );
    Ok(())
}

fn evaluate_command(run_dir: &Path, arguments: &[String]) -> io::Result<()> {
    let simulations = option(arguments, "--simulations")
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_SIMULATIONS);
    let pairs = option(arguments, "--pairs")
        .and_then(|value| value.parse().ok())
        .unwrap_or(64);
    let run = load_run(run_dir, false)?;
    let result = arena(
        &run.model,
        &run.champion,
        simulations,
        run.games + 1,
        pairs,
        default_workers(),
    );
    println!(
        "latest vs staged champion: score={:.3}, lower99={:.3}, games={}, base simulations/action={simulations}",
        result.score, result.lower_bound, result.games
    );
    Ok(())
}

fn publish(run_dir: &Path, allow_when_dirty: bool) -> io::Result<()> {
    agent_publisher::require_clean_worktree(Path::new("."), allow_when_dirty)?;
    publish_to(
        run_dir,
        Path::new("crates/agent-cherry/model/cherry.bin"),
        Path::new("web"),
    )
}

fn publish_to(run_dir: &Path, destination: &Path, web_directory: &Path) -> io::Result<()> {
    let (_, _, _, validated_protocol) = load_meta(&run_dir.join("state.txt"))?;
    if !validated_protocol {
        return Err(io::Error::other(
            "run predates the robust promotion protocol; resume training before publishing",
        ));
    }
    let source = run_dir.join("champion.bin");
    let champion = Model::load(&source)?;
    let latest = Model::load(run_dir.join("latest.bin"))?;
    let Some(parent) = destination.parent() else {
        return Err(io::Error::other("invalid publication path"));
    };
    fs::create_dir_all(parent)?;
    let publication =
        agent_publisher::publish_model(destination, &champion.to_bytes(), web_directory)?;
    println!(
        "Published validated Cherry champion step {} from {} to {} (latest unvalidated training step {}); web version {} -> {}",
        champion.training_steps,
        source.display(),
        destination.display(),
        latest.training_steps,
        publication.previous_version,
        publication.version,
    );
    println!("Rebuild WASM to load the new checkpoint.");
    Ok(())
}

fn status(run_dir: &Path) -> io::Result<()> {
    let run = load_run(run_dir, false)?;
    println!("run={}", absolute(run_dir)?.display());
    println!("games={}", run.games);
    println!("latest_training_steps={}", run.model.training_steps);
    println!("champion_training_steps={}", run.champion.training_steps);
    println!("replay_positions={}", run.replay.len());
    println!("validated_promotions={}", run.promotions);
    println!(
        "latest={}",
        absolute(&run_dir.join("latest.bin"))?.display()
    );
    Ok(())
}

fn load_run(run_dir: &Path, prepare_training: bool) -> io::Result<RunState> {
    let latest = run_dir.join("latest.bin");
    let model = if latest.exists() {
        Model::load(&latest)?
    } else {
        Model::seeded(0xC4E2_9917_D15C_A11E)
    };
    let (games, promotions, rng_seed, validated_protocol) = load_meta(&run_dir.join("state.txt"))?;
    let champion_path = run_dir.join("champion.bin");
    let champion = if validated_protocol && champion_path.exists() {
        Model::load(champion_path)?
    } else {
        // Rebootstrap from the current network instead of trusting a champion
        // selected by the historical four-game promotion protocol.
        model.clone()
    };
    let replay = load_run_replay(&run_dir.join("replay.bin"), prepare_training)?;
    let optimizer_path = run_dir.join("optimizer.bin");
    let optimizer = if validated_protocol && optimizer_path.exists() {
        Adam::from_bytes(&fs::read(&optimizer_path)?)?
    } else {
        if prepare_training && !validated_protocol && optimizer_path.exists() {
            let backup = quarantine_file(&optimizer_path, "optimizer-v1-corrupt")?;
            println!(
                "Quarantined legacy Adam moments at {}; starting a clean optimizer.",
                backup.display()
            );
        }
        Adam::new()
    };
    Ok(RunState {
        model,
        champion,
        optimizer,
        replay,
        games,
        promotions,
        rng: Rng::new(rng_seed),
    })
}

fn load_run_replay(path: &Path, quarantine_legacy: bool) -> io::Result<ReplayBuffer> {
    if !path.exists() {
        return Ok(ReplayBuffer::new());
    }
    let mut file = fs::File::open(path)?;
    let mut magic = [0; 8];
    file.read_exact(&mut magic)?;
    if &magic != REPLAY_MAGIC_V1 {
        return load_replay(path);
    }
    drop(file);
    if !quarantine_legacy {
        println!("Ignoring corrupt legacy policy replay; training will quarantine it.");
        return Ok(ReplayBuffer::new());
    }

    let backup = quarantine_file(path, "replay-v1-corrupt")?;
    println!(
        "Quarantined corrupt legacy policy replay at {}; starting a clean compact replay.",
        backup.display()
    );
    Ok(ReplayBuffer::new())
}

fn quarantine_file(path: &Path, backup_stem: &str) -> io::Result<PathBuf> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let backup = (0_u64..)
        .map(|suffix| {
            if suffix == 0 {
                parent.join(format!("{backup_stem}.bin"))
            } else {
                parent.join(format!("{backup_stem}-{suffix}.bin"))
            }
        })
        .find(|candidate| !candidate.exists())
        .expect("an available replay backup filename exists");
    fs::rename(path, &backup)?;
    Ok(backup)
}

fn save_run(
    run_dir: &Path,
    run: &RunState,
    loss: f32,
    arena: Option<ArenaResult>,
    save_replay: bool,
) -> io::Result<()> {
    atomic_write(&run_dir.join("latest.bin"), &run.model.to_bytes())?;
    atomic_write(&run_dir.join("champion.bin"), &run.champion.to_bytes())?;
    if save_replay {
        save_replay_file(&run_dir.join("replay.bin"), &run.replay)?;
    }
    atomic_write(&run_dir.join("optimizer.bin"), &run.optimizer.to_bytes())?;
    let metadata = format!(
        "games={}\nvalidated_promotions={}\npromotion_protocol=2\nrng={}\ntraining_steps={}\nreplay_positions={}\nlast_loss={loss}\nlast_arena={}\nupdated_unix={}\n",
        run.games,
        run.promotions,
        run.rng.0,
        run.model.training_steps,
        run.replay.len(),
        arena.map_or_else(
            || "not-run".to_owned(),
            |result| format!(
                "score:{:.6},lower99:{:.6},games:{}",
                result.score, result.lower_bound, result.games
            )
        ),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );
    atomic_write(&run_dir.join("state.txt"), metadata.as_bytes())
}

fn archive_champion(run_dir: &Path, run: &RunState) -> io::Result<()> {
    let league = run_dir.join("validated-league-v2");
    fs::create_dir_all(&league)?;
    run.champion.save(league.join(format!(
        "champion-{:04}-step-{}.bin",
        run.promotions, run.champion.training_steps
    )))
}

fn append_arena_report(run_dir: &Path, run: &RunState, result: ArenaResult) -> io::Result<()> {
    let path = run_dir.join("arena-v2.csv");
    let is_new = !path.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if is_new {
        writeln!(
            file,
            "games,training_steps,validated_promotions,score,lower99,arena_games"
        )?;
    }
    writeln!(
        file,
        "{},{},{},{:.6},{:.6},{}",
        run.games,
        run.model.training_steps,
        run.promotions,
        result.score,
        result.lower_bound,
        result.games
    )
}

fn passes_league_guard(
    run_dir: &Path,
    candidate: &Model,
    incumbent: &Model,
    simulations: usize,
    round: u64,
    workers: usize,
) -> io::Result<bool> {
    // Deliberately ignore the legacy `league/` directory: those checkpoints
    // include promotions decided by four-game arenas and are not trustworthy
    // strength anchors.
    let league = run_dir.join("validated-league-v2");
    if !league.exists() {
        return Ok(true);
    }
    let mut checkpoints = fs::read_dir(league)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "bin"))
        .collect::<Vec<_>>();
    checkpoints.sort();
    let checkpoints = checkpoints.into_iter().rev().take(8).collect::<Vec<_>>();
    if checkpoints.is_empty() {
        return Ok(true);
    }
    let mut candidate_score = 0.0;
    let mut incumbent_score = 0.0;
    for (index, path) in checkpoints.iter().enumerate() {
        let anchor = Model::load(path)?;
        let arena_round = round ^ (index as u64).rotate_left(29);
        candidate_score += arena(candidate, &anchor, simulations, arena_round, 24, workers).score;
        incumbent_score += arena(incumbent, &anchor, simulations, arena_round, 24, workers).score;
    }
    candidate_score /= checkpoints.len() as f32;
    incumbent_score /= checkpoints.len() as f32;
    Ok(candidate_score + 0.01 >= incumbent_score)
}

fn load_meta(path: &Path) -> io::Result<(u64, u64, u64, bool)> {
    if !path.exists() {
        return Ok((0, 0, 0x51A7_E5E5_C4E2_0001, false));
    }
    let text = fs::read_to_string(path)?;
    let get = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0)
    };
    Ok((
        get("games"),
        get("validated_promotions"),
        get("rng").max(1),
        get("promotion_protocol") == 2,
    ))
}

fn save_replay_file(path: &Path, replay: &ReplayBuffer) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    let file = fs::File::create(&temporary)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(REPLAY_MAGIC_V2)?;
    writer.write_all(&(replay.len() as u64).to_le_bytes())?;
    for sample in replay.chronological() {
        writer.write_all(sample.input.as_slice())?;
        let policy_len = u16::try_from(sample.policy.len()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "replay policy is too large")
        })?;
        writer.write_all(&policy_len.to_le_bytes())?;
        for &(index, weight) in sample.policy.iter() {
            writer.write_all(&index.to_le_bytes())?;
            writer.write_all(&weight.to_le_bytes())?;
        }
        writer.write_all(&sample.value.to_le_bytes())?;
    }
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    fs::rename(temporary, path)
}

fn load_replay(path: &Path) -> io::Result<ReplayBuffer> {
    let mut reader = BufReader::new(fs::File::open(path)?);
    let mut magic = [0; 8];
    reader.read_exact(&mut magic)?;
    let count = read_u64(&mut reader)?;
    if count > 10_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cherry replay has an unreasonable sample count",
        ));
    }
    if &magic != REPLAY_MAGIC_V2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid compact Cherry replay",
        ));
    }
    let mut replay = ReplayBuffer::new();
    load_replay_v2(&mut reader, count as usize, &mut replay)?;
    let mut trailing = [0];
    if reader.read(&mut trailing)? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Cherry replay contains trailing data",
        ));
    }
    Ok(replay)
}

fn load_replay_v2(
    reader: &mut impl Read,
    count: usize,
    replay: &mut ReplayBuffer,
) -> io::Result<()> {
    for _ in 0..count {
        let mut input = [0; PACKED_INPUT_SIZE];
        reader.read_exact(&mut input)?;
        let policy_len = usize::from(read_u16(reader)?);
        if policy_len == 0 || policy_len > ACTION_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "replay policy has too many entries",
            ));
        }
        let mut policy = Vec::with_capacity(policy_len);
        let mut seen_actions = [false; ACTION_SIZE];
        for _ in 0..policy_len {
            let index = read_u16(reader)?;
            let weight = read_u16(reader)?;
            if usize::from(index) >= ACTION_SIZE || weight == 0 || seen_actions[usize::from(index)]
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "replay contains an invalid sparse policy",
                ));
            }
            seen_actions[usize::from(index)] = true;
            policy.push((index, weight));
        }
        let mut value = [0];
        reader.read_exact(&mut value)?;
        let value = i8::from_le_bytes(value);
        if !(-1..=1).contains(&value) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "replay contains an invalid outcome",
            ));
        }
        replay.push(CompactSample {
            input,
            policy: policy.into_boxed_slice(),
            value,
        });
    }
    Ok(())
}

fn read_u16(reader: &mut impl Read) -> io::Result<u16> {
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u64(reader: &mut impl Read) -> io::Result<u64> {
    let mut bytes = [0; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, path)
}

fn absolute(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1_u32 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    fn temporary_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!(
            "cherry-train-{name}-{}-{}.bin",
            std::process::id(),
            NEXT_PATH.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn temporary_dir(name: &str) -> PathBuf {
        let path = temporary_path(name).with_extension("dir");
        fs::create_dir(&path).unwrap();
        path
    }

    fn compact_sample(marker: u8) -> CompactSample {
        let mut input = [0; PACKED_INPUT_SIZE];
        set_packed_feature(&mut input, 0, marker % 3);
        CompactSample {
            input,
            policy: vec![(usize::from(marker).min(ACTION_SIZE - 1) as u16, u16::MAX)]
                .into_boxed_slice(),
            value: (marker % 3) as i8 - 1,
        }
    }

    #[test]
    fn adaptive_budget_covers_wide_roots_without_capping_explicit_depth() {
        assert_eq!(adaptive_simulations(24, 290), 870);
        assert_eq!(adaptive_simulations(256, 100), 300);
        assert_eq!(adaptive_simulations(2_000, 290), 2_000);
    }

    #[test]
    fn progress_reports_default_to_on_and_accept_explicit_toggles() {
        let arguments = strings(&["train"]);
        assert!(parse_on_off(&arguments, "--progress-reports", true).unwrap());

        let arguments = strings(&["train", "--progress-reports", "OFF"]);
        assert!(!parse_on_off(&arguments, "--progress-reports", true).unwrap());

        let arguments = strings(&["train", "--progress-reports", "yes"]);
        assert!(parse_on_off(&arguments, "--progress-reports", false).unwrap());
    }

    #[test]
    fn progress_report_toggle_rejects_missing_and_invalid_values() {
        let missing = strings(&["train", "--progress-reports"]);
        assert!(
            parse_on_off(&missing, "--progress-reports", true)
                .unwrap_err()
                .to_string()
                .contains("requires on or off")
        );

        let invalid = strings(&["train", "--progress-reports", "sometimes"]);
        assert!(
            parse_on_off(&invalid, "--progress-reports", true)
                .unwrap_err()
                .to_string()
                .contains("expected on or off")
        );
    }

    #[test]
    fn progress_reports_use_readable_utc_timestamps_and_counts() {
        assert_eq!(utc_timestamp(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        assert_eq!(
            utc_timestamp(UNIX_EPOCH + Duration::from_secs(1_000_000_000)),
            "2001-09-09T01:46:40Z"
        );
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(192_025), "192,025");
        assert_eq!(
            timestamped_report(
                UNIX_EPOCH + Duration::from_secs(1_000_000_000),
                "Cherry is training."
            ),
            "[2001-09-09T01:46:40Z] Cherry is training."
        );
    }

    #[test]
    fn arena_reports_explain_promotions_and_rejections() {
        let result = ArenaResult {
            score: 0.625,
            lower_bound: 0.538,
            games: 192,
        };
        let promoted = format_arena_report(
            progress_snapshot(160_000, 960_000, 960_000, 38),
            960_000,
            954_000,
            result,
            ArenaDecision::Promoted,
        );
        assert!(promoted.contains("Promotion 38 at game 160,000"));
        assert!(promoted.contains("scored 62.5% (53.8% lower 99% confidence bound)"));

        let rejected = format_arena_report(
            progress_snapshot(161_000, 966_000, 960_000, 38),
            966_000,
            960_000,
            result,
            ArenaDecision::LeagueGuardRejected,
        );
        assert!(rejected.contains("did not pass the validated league guard"));
        assert!(rejected.contains("No promotion occurred; training continues"));

        let continued = format_arena_report(
            progress_snapshot(162_000, 972_000, 960_000, 38),
            972_000,
            960_000,
            ArenaResult {
                score: 0.531,
                lower_bound: 0.445,
                games: 192,
            },
            ArenaDecision::Continued,
        );
        assert!(continued.contains("No promotion occurred; training continues"));
    }

    #[test]
    fn circular_replay_keeps_newest_samples_in_chronological_order() {
        let mut replay = ReplayBuffer::with_capacity(3);
        for marker in 0..5 {
            replay.push(compact_sample(marker));
        }
        let markers = replay
            .chronological()
            .map(|sample| sample.policy[0].0)
            .collect::<Vec<_>>();
        assert_eq!(markers, [2, 3, 4]);
    }

    #[test]
    fn compact_replay_round_trips_and_expands_a_training_batch() {
        let path = temporary_path("round-trip");
        let mut replay = ReplayBuffer::with_capacity(3);
        for marker in 0..5 {
            replay.push(compact_sample(marker));
        }
        save_replay_file(&path, &replay).unwrap();
        let loaded = load_replay(&path).unwrap();
        fs::remove_file(&path).unwrap();

        let markers = loaded
            .chronological()
            .map(|sample| sample.policy[0].0)
            .collect::<Vec<_>>();
        assert_eq!(markers, [2, 3, 4]);
        let mut rng = Rng::new(7);
        let mut batch = Vec::new();
        fill_random_batch(&loaded, 2, &mut rng, &mut batch);
        assert_eq!(batch.len(), 2);
        for sample in batch {
            assert!(sample.input[0] >= 1.0);
            assert_eq!(sample.policy.iter().sum::<f32>(), 1.0);
            assert!((-1.0..=1.0).contains(&sample.value));
        }
    }

    #[test]
    fn legacy_dense_replay_is_quarantined_instead_of_trained_on() {
        let directory = temporary_dir("legacy");
        let path = directory.join("replay.bin");
        fs::write(&path, REPLAY_MAGIC_V1).unwrap();

        let loaded = load_run_replay(&path, true).unwrap();
        assert!(loaded.is_empty());
        assert!(!path.exists());
        assert!(directory.join("replay-v1-corrupt.bin").exists());
        fs::remove_file(directory.join("replay-v1-corrupt.bin")).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn legacy_promotion_count_is_not_treated_as_validated() {
        let path = temporary_path("legacy-meta");
        fs::write(&path, b"games=80\npromotions=20\nrng=9\n").unwrap();
        let (games, promotions, rng, validated_protocol) = load_meta(&path).unwrap();
        fs::remove_file(&path).unwrap();
        assert_eq!((games, promotions, rng), (80, 0, 9));
        assert!(!validated_protocol);
    }

    #[test]
    fn publish_exports_the_validated_champion_and_advances_the_web_version() {
        let root = temporary_dir("publish");
        let run_dir = root.join("run");
        let web_directory = root.join("web");
        let destination = root.join("published/cherry.bin");
        fs::create_dir_all(&run_dir).unwrap();
        write_test_web_manifests(&web_directory);

        let mut champion = Model::seeded(7);
        champion.training_steps = 80_000;
        champion.save(run_dir.join("champion.bin")).unwrap();
        let mut latest = Model::seeded(7);
        latest.training_steps = 80_100;
        latest.save(run_dir.join("latest.bin")).unwrap();
        fs::write(
            run_dir.join("state.txt"),
            "games=100\nvalidated_promotions=2\npromotion_protocol=2\nrng=9\n",
        )
        .unwrap();

        publish_to(&run_dir, &destination, &web_directory).unwrap();

        let published = Model::load(destination).unwrap();
        assert_eq!(published.training_steps, champion.training_steps);
        assert_eq!(published.to_bytes(), champion.to_bytes());
        assert!(
            fs::read_to_string(web_directory.join("package.json"))
                .unwrap()
                .contains("\"version\": \"0.2.0\"")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn write_test_web_manifests(web_directory: &Path) {
        fs::create_dir_all(web_directory).unwrap();
        fs::write(
            web_directory.join("package.json"),
            "{\n  \"name\": \"web\",\n  \"version\": \"0.1.7\"\n}\n",
        )
        .unwrap();
        fs::write(
            web_directory.join("package-lock.json"),
            "{\n  \"name\": \"web\",\n  \"version\": \"0.1.7\",\n  \"packages\": {\n    \"\": {\n      \"name\": \"web\",\n      \"version\": \"0.1.7\"\n    }\n  }\n}\n",
        )
        .unwrap();
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn progress_snapshot(
        games: u64,
        candidate_steps: u64,
        champion_steps: u64,
        promotions: u64,
    ) -> ProgressSnapshot {
        ProgressSnapshot {
            games,
            candidate_steps,
            champion_steps,
            promotions,
            replay_positions: 500_000,
        }
    }
}
