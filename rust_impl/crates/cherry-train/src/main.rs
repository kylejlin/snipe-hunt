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
            eprintln!("Cherry trainer error: {error}");
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
            train(
                &run_dir,
                Duration::from_secs_f64((hours * 3600.0).max(1.0)),
                simulations,
                workers,
            )
        }
        "publish" => publish(&run_dir),
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

fn help() {
    println!(
        "Cherry self-play trainer\n\
         \n\
         train          [--run-dir PATH] [--hours N] [--simulations N] [--workers N]\n\
         status        [--run-dir PATH]\n\
         evaluate      [--run-dir PATH] [--simulations N]\n\
         audit         [--run-dir PATH] [--simulations N] [--pairs N]\n\
         publish       [--run-dir PATH]\n\
         \n\
         Simulations is a base; wide positions automatically receive at least 3x legal actions.\n\
         Weights and optimizer checkpoint after every completed game; compact replay every 25.\n\
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

fn train(run_dir: &Path, duration: Duration, simulations: usize, workers: usize) -> io::Result<()> {
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
    println!(
        "Cherry training resumed: games={}, steps={}, replay={}, base simulations/action={}, workers={}",
        run.games,
        run.model.training_steps,
        run.replay.len(),
        simulations,
        workers
    );
    println!("Run directory: {}", absolute(run_dir)?.display());
    println!("Press Ctrl+C once for a graceful stop and full checkpoint.");

    while started.elapsed() < duration && !shutdown_requested() {
        let batch_started = Instant::now();
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
                if run.games % PROMOTION_INTERVAL == 0 {
                    let result = arena(
                        &run.model,
                        &run.champion,
                        simulations,
                        run.games,
                        PROMOTION_PAIRS,
                        workers,
                    );
                    if result.lower_bound > 0.5
                        && passes_league_guard(
                            run_dir,
                            &run.model,
                            &run.champion,
                            simulations.max(8),
                            run.games,
                            workers,
                        )?
                    {
                        run.champion = run.model.clone();
                        run.promotions += 1;
                        archive_champion(run_dir, &run)?;
                    }
                    append_arena_report(run_dir, &run, result)?;
                    arena_result = Some(result);
                }
                last_arena = arena_result;
                save_run(
                    run_dir,
                    &run,
                    loss,
                    arena_result,
                    run.games < 10 || run.games % REPLAY_SAVE_INTERVAL == 0,
                )?;
                println!(
                    "game {:>6}  winner={:<5} actions={:<3} replay={:<5} loss={:.4} batch={:.1}s{}",
                    run.games,
                    winner.map_or("draw", |player| if player == Player::Alpha {
                        "Alpha"
                    } else {
                        "Beta"
                    }),
                    actions,
                    run.replay.len(),
                    loss,
                    batch_started.elapsed().as_secs_f32(),
                    arena_result.map_or_else(String::new, |result| format!(
                        " arena={:.3} lower99={:.3}",
                        result.score, result.lower_bound
                    )),
                );
            }
            Ok::<(), io::Error>(())
        })?;
    }
    save_run(run_dir, &run, last_loss, last_arena, true)?;
    if shutdown_requested() {
        println!(
            "Cherry stopped gracefully at game {} and step {}. Full checkpoint saved to {}.",
            run.games,
            run.model.training_steps,
            absolute(run_dir)?.display()
        );
    } else {
        println!(
            "Training window complete: games={}, steps={}, promotions={}",
            run.games, run.model.training_steps, run.promotions
        );
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

fn publish(run_dir: &Path) -> io::Result<()> {
    let (_, _, _, validated_protocol) = load_meta(&run_dir.join("state.txt"))?;
    if !validated_protocol {
        return Err(io::Error::other(
            "run predates the robust promotion protocol; resume training before publishing",
        ));
    }
    let source = run_dir.join("champion.bin");
    let champion = Model::load(&source)?;
    let latest = Model::load(run_dir.join("latest.bin"))?;
    let destination = PathBuf::from("crates/agent-cherry/model/cherry.bin");
    let Some(parent) = destination.parent() else {
        return Err(io::Error::other("invalid publication path"));
    };
    fs::create_dir_all(parent)?;
    atomic_write(&destination, &champion.to_bytes())?;
    println!(
        "Published validated Cherry champion step {} from {} to {} (latest unvalidated training step {})",
        champion.training_steps,
        source.display(),
        destination.display(),
        latest.training_steps,
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
    println!(
        "league guard: candidate={candidate_score:.3}, incumbent={incumbent_score:.3}, checkpoints={}",
        checkpoints.len()
    );
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
}
