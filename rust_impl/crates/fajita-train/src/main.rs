use agent_fajita::{
    ACTION_SIZE, INITIAL_SEED, INPUT_SIZE, Model, Search, action_index, encode_state, state_key,
    training::{Adam, Sample},
};
use snipe_core::{Action, Player, State};
use snipe_prng::initial_state;
use std::{
    collections::HashSet,
    env, fs,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEFAULT_RUN_DIR: &str = "training/fajita-main";
const DEFAULT_HOURS: f64 = 8.0;
const DEFAULT_SIMULATIONS: usize = 512;
const MAX_REPLAY: usize = 500_000;
const MAX_ATOMIC_ACTIONS: usize = 256;
const BATCH_SIZE: usize = 48;
const UPDATES_PER_GAME: usize = 4;
const CHECKPOINT_INTERVAL: u64 = 25;
const REPLAY_SAVE_INTERVAL: u64 = 500;
const REPORT_INTERVAL: u64 = 100;
const PROMOTION_INTERVAL: u64 = 1_000;
const PROMOTION_PAIRS: usize = 48;
const DIRICHLET_ALPHA: f32 = 0.3;
const ROOT_NOISE_FRACTION: f32 = 0.25;
const INITIAL_LEARNING_RATE: f32 = 0.00035;
const MATURE_LEARNING_RATE: f32 = 0.000175;
const MATURE_STEP: u64 = 150_000;
const REGRESSION_SCORE: f32 = 0.45;
const PACKED_INPUT_SIZE: usize = INPUT_SIZE.div_ceil(4);
const REPLAY_MAGIC: &[u8; 8] = b"FAJRPL01";

fn main() -> ExitCode {
    match run() {
        Ok(success) if success => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(2),
        Err(error) => {
            eprintln!("fajita-train: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> io::Result<bool> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let command = arguments.first().map(String::as_str).unwrap_or("train");
    let run_dir = option(&arguments, "--run-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_RUN_DIR));
    match command {
        "train" | "nightly" => {
            let hours = parse_option(&arguments, "--hours", DEFAULT_HOURS)?;
            let simulations = parse_option(&arguments, "--simulations", DEFAULT_SIMULATIONS)?;
            let workers = parse_option(&arguments, "--workers", default_workers())?;
            train(
                &run_dir,
                Duration::from_secs_f64((hours * 3600.0).max(1.0)),
                simulations,
                workers,
            )?;
            Ok(true)
        }
        "status" => {
            status(&run_dir)?;
            Ok(true)
        }
        "evaluate" => {
            let pairs = parse_option(&arguments, "--pairs", 24)?;
            let simulations = parse_option(&arguments, "--simulations", DEFAULT_SIMULATIONS)?;
            evaluate(&run_dir, pairs, simulations)?;
            Ok(true)
        }
        "recover" => {
            recover(&run_dir)?;
            Ok(true)
        }
        "publish" => {
            publish(&run_dir)?;
            Ok(true)
        }
        "help" | "--help" | "-h" => {
            help();
            Ok(true)
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command {other:?}; use `fajita-train help`"),
        )),
    }
}

fn option<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn parse_option<T>(arguments: &[String], name: &str, default: T) -> io::Result<T>
where
    T: std::str::FromStr,
{
    option(arguments, name).map_or(Ok(default), |value| {
        value.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid value {value:?} for {name}"),
            )
        })
    })
}

fn help() {
    println!(
        "Fajita rules-only self-play trainer\n\
         \n\
         train|nightly [--run-dir PATH] [--hours N] [--simulations N] [--workers N]\n\
         status        [--run-dir PATH]\n\
         evaluate      [--run-dir PATH] [--pairs N] [--simulations N]\n\
         recover       [--run-dir PATH]\n\
         publish       [--run-dir PATH]\n\
         \n\
         A new run always starts from deterministic fresh weights. Training consumes only\n\
         legal self-play positions, MCTS visit policies, and final game outcomes.\n\
         The default 512 base and wide-root scaling exactly match Cherry's trainer."
    );
}

struct RunState {
    model: Model,
    champion: Model,
    optimizer: Adam,
    replay: ReplayBuffer,
    games: u64,
    promotions: u64,
    recoveries: u64,
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
        Self {
            entries: Vec::with_capacity(MAX_REPLAY),
            next: 0,
            capacity: MAX_REPLAY,
        }
    }

    #[cfg(test)]
    fn with_capacity(capacity: usize) -> Self {
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
    let mut run = load_run(run_dir)?;
    let started = Instant::now();
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    println!(
        "Fajita self-play resumed: games={}, steps={}, replay={}, base simulations/action={}, workers={}",
        run.games,
        run.model.training_steps,
        run.replay.len(),
        simulations,
        workers
    );
    println!("Run directory: {}", absolute(run_dir)?.display());
    let mut last_loss = 0.0;
    let mut last_arena = None;

    while started.elapsed() < duration {
        let generation_started = Instant::now();
        let snapshot = run.model.clone();
        let jobs = (0..workers)
            .map(|_| (run.rng.next_u64(), run.rng.next_u64()))
            .collect::<Vec<_>>();
        let (sender, receiver) = mpsc::channel();
        thread::scope(|scope| -> io::Result<()> {
            for (deal_seed, random_seed) in jobs {
                let sender = sender.clone();
                let model = &snapshot;
                scope.spawn(move || {
                    let mut rng = Rng::new(random_seed);
                    let game = self_play(model, deal_seed, simulations, &mut rng);
                    sender.send(game).expect("training receiver remains alive");
                });
            }
            drop(sender);
            for game in receiver {
                run.replay.extend(game.samples);
                run.games += 1;
                let mut loss = 0.0;
                if !run.replay.is_empty() {
                    for _ in 0..UPDATES_PER_GAME {
                        fill_random_batch(&run.replay, BATCH_SIZE, &mut run.rng, &mut batch);
                        let rate = learning_rate(run.model.training_steps);
                        loss += run.model.train_batch(&batch, &mut run.optimizer, rate);
                    }
                    loss /= UPDATES_PER_GAME as f32;
                }
                last_loss = loss;

                let mut arena = None;
                let mut recovered = false;
                if run.games % PROMOTION_INTERVAL == 0 {
                    let result = paired_arena(
                        &run.model,
                        &run.champion,
                        simulations,
                        run.games,
                        PROMOTION_PAIRS,
                    );
                    if result.lower_95 > 0.5 {
                        run.champion = run.model.clone();
                        run.promotions += 1;
                        archive_champion(run_dir, &run)?;
                    }
                    append_arena(run_dir, &run, result)?;
                    if result.score < REGRESSION_SCORE
                        && run.model.training_steps != run.champion.training_steps
                    {
                        run.model = run.champion.clone();
                        run.optimizer = Adam::new();
                        run.recoveries += 1;
                        recovered = true;
                    }
                    arena = Some(result);
                    last_arena = arena;
                }
                if run.games == 1 || run.games % CHECKPOINT_INTERVAL == 0 {
                    save_run(
                        run_dir,
                        &run,
                        loss,
                        arena,
                        run.games == 1 || run.games % REPLAY_SAVE_INTERVAL == 0,
                    )?;
                }
                if run.games <= 10 || run.games % REPORT_INTERVAL == 0 || arena.is_some() {
                    println!(
                        "game {:>7} winner={:<5} actions={:<3} replay={:<6} loss={:.4} elapsed={:.1}s{}",
                        run.games,
                        game.winner
                            .map_or("draw", |winner| if winner == Player::Alpha {
                                "Alpha"
                            } else {
                                "Beta"
                            }),
                        game.actions,
                        run.replay.len(),
                        loss,
                        generation_started.elapsed().as_secs_f32(),
                        arena.map_or_else(String::new, |result| {
                            format!(
                                " arena={:.3} lower95={:.3}{}",
                                result.score,
                                result.lower_95,
                                if recovered {
                                    format!(" recovered_to_step={}", run.model.training_steps)
                                } else {
                                    String::new()
                                }
                            )
                        })
                    );
                }
            }
            Ok(())
        })?;
    }
    save_run(run_dir, &run, last_loss, last_arena, true)?;
    println!(
        "training window complete: games={}, steps={}, promotions={}",
        run.games, run.model.training_steps, run.promotions
    );
    Ok(())
}

fn learning_rate(training_steps: u64) -> f32 {
    if training_steps < MATURE_STEP {
        INITIAL_LEARNING_RATE
    } else {
        MATURE_LEARNING_RATE
    }
}

struct SelfPlayGame {
    samples: Vec<CompactSample>,
    winner: Option<Player>,
    actions: usize,
}

fn self_play(model: &Model, deal_seed: u64, simulations: usize, rng: &mut Rng) -> SelfPlayGame {
    struct Pending {
        input: [u8; PACKED_INPUT_SIZE],
        policy: Box<[(u16, u16)]>,
        player: Player,
    }

    let mut state = initial_state(deal_seed);
    let mut search = Search::new(state.clone(), model);
    let mut pending = Vec::new();
    let mut seen = HashSet::new();
    let mut winner = None;
    for action_number in 0..MAX_ATOMIC_ACTIONS {
        if let Some(found) = state.winner() {
            winner = Some(found);
            break;
        }
        if !seen.insert(state_key(&state)) {
            break;
        }
        search.add_root_noise(DIRICHLET_ALPHA, ROOT_NOISE_FRACTION, rng.next_u64());
        search.simulate_n(
            model,
            adaptive_simulations(simulations, search.root_action_count()),
        );
        let policy = search.policy(if action_number < 24 { 1.0 } else { 0.08 });
        if policy.is_empty() {
            winner = state.winner();
            break;
        }
        pending.push(Pending {
            input: compact_input(&encode_state(&state)),
            policy: compact_policy(&state, &policy),
            player: state.active_player,
        });
        let action = sample_action(&policy, rng);
        state = state
            .apply(action)
            .expect("Fajita self-play selected a legal action");
        if !search.advance(action, model) {
            search = Search::new(state.clone(), model);
        }
    }
    winner = winner.or_else(|| state.winner());
    let actions = pending.len();
    let samples = pending
        .into_iter()
        .map(|sample| CompactSample {
            input: sample.input,
            policy: sample.policy,
            value: winner.map_or(0, |won| if won == sample.player { 1 } else { -1 }),
        })
        .collect();
    SelfPlayGame {
        samples,
        winner,
        actions,
    }
}

fn adaptive_simulations(base: usize, branching_factor: usize) -> usize {
    base.max(branching_factor.saturating_mul(3).min(1_536))
}

fn sample_action(policy: &[(Action, f32)], rng: &mut Rng) -> Action {
    let target = rng.unit();
    let mut cumulative = 0.0;
    for &(action, probability) in policy {
        cumulative += probability;
        if target <= cumulative {
            return action;
        }
    }
    policy.last().expect("policy is non-empty").0
}

fn compact_policy(state: &State, policy: &[(Action, f32)]) -> Box<[(u16, u16)]> {
    policy
        .iter()
        .filter_map(|&(action, probability)| {
            let weight = (probability * f32::from(u16::MAX)).round() as u16;
            (weight > 0).then_some((
                u16::try_from(action_index(state, action)).expect("action index fits u16"),
                weight,
            ))
        })
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn compact_input(input: &[f32; INPUT_SIZE]) -> [u8; PACKED_INPUT_SIZE] {
    let mut packed = [0; PACKED_INPUT_SIZE];
    for (index, value) in input.iter().copied().enumerate() {
        let quantized = (value * 2.0).round() as u8;
        debug_assert!(quantized <= 2);
        set_packed_feature(&mut packed, index, quantized);
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
    batch.resize_with(size.min(replay.len()), || Sample {
        input: [0.0; INPUT_SIZE],
        policy: [0.0; ACTION_SIZE],
        value: 0.0,
    });
    for output in batch {
        let sample = replay.get((rng.next_u64() as usize) % replay.len());
        for (index, feature) in output.input.iter_mut().enumerate() {
            *feature = f32::from(packed_feature(&sample.input, index)) * 0.5;
        }
        output.policy.fill(0.0);
        let total = sample
            .policy
            .iter()
            .map(|(_, weight)| u32::from(*weight))
            .sum::<u32>()
            .max(1);
        for &(index, weight) in sample.policy.iter() {
            output.policy[usize::from(index)] = f32::from(weight) / total as f32;
        }
        output.value = f32::from(sample.value);
    }
}

#[derive(Clone, Copy)]
struct ArenaResult {
    score: f32,
    lower_95: f32,
    games: usize,
}

fn paired_arena(
    candidate: &Model,
    incumbent: &Model,
    simulations: usize,
    round: u64,
    pairs: usize,
) -> ArenaResult {
    let worker_count = default_workers().min(pairs.max(1));
    let (sender, receiver) = mpsc::channel();
    thread::scope(|scope| {
        for worker in 0..worker_count {
            let sender = sender.clone();
            scope.spawn(move || {
                for pair in (worker..pairs).step_by(worker_count) {
                    let seed = 0xFA71_A000_0000_0000 ^ round.rotate_left(19) ^ pair as u64;
                    let as_alpha =
                        model_match(candidate, incumbent, seed, simulations, simulations);
                    let as_beta =
                        1.0 - model_match(incumbent, candidate, seed, simulations, simulations);
                    sender
                        .send((pair, (as_alpha + as_beta) * 0.5))
                        .expect("arena receiver remains alive");
                }
            });
        }
        drop(sender);
    });
    let mut scores = vec![0.0; pairs];
    for (pair, score) in receiver {
        scores[pair] = score;
    }
    let score = scores.iter().sum::<f32>() / pairs.max(1) as f32;
    let variance = if pairs > 1 {
        scores
            .iter()
            .map(|paired| (paired - score).powi(2))
            .sum::<f32>()
            / (pairs - 1) as f32
    } else {
        0.25
    };
    let standard_error = (variance / pairs.max(1) as f32).sqrt();
    ArenaResult {
        score,
        lower_95: (score - 1.645 * standard_error).clamp(0.0, 1.0),
        games: pairs * 2,
    }
}

/// Returns Alpha's score. Repeated and capped games are neutral.
fn model_match(
    alpha: &Model,
    beta: &Model,
    seed: u64,
    alpha_simulations: usize,
    beta_simulations: usize,
) -> f32 {
    let mut state = initial_state(seed);
    let mut seen = HashSet::new();
    for _ in 0..MAX_ATOMIC_ACTIONS {
        if let Some(winner) = state.winner() {
            return f32::from(winner == Player::Alpha);
        }
        if !seen.insert(state_key(&state)) {
            return 0.5;
        }
        let model = if state.active_player == Player::Alpha {
            alpha
        } else {
            beta
        };
        let budget = if state.active_player == Player::Alpha {
            alpha_simulations
        } else {
            beta_simulations
        };
        let mut search = Search::new(state.clone(), model);
        search.simulate_n(
            model,
            adaptive_simulations(budget, search.root_action_count()),
        );
        let Some((action, _)) = search
            .policy(0.0)
            .into_iter()
            .max_by(|left, right| left.1.total_cmp(&right.1))
        else {
            return 0.5;
        };
        state = state.apply(action).expect("arena selected a legal action");
    }
    0.5
}

fn evaluate(run_dir: &Path, pairs: usize, simulations: usize) -> io::Result<()> {
    let run = load_run(run_dir)?;
    let result = paired_arena(&run.model, &run.champion, simulations, run.games, pairs);
    println!(
        "latest step {} vs champion step {}: score={:.3}, lower95={:.3}, games={}",
        run.model.training_steps,
        run.champion.training_steps,
        result.score,
        result.lower_95,
        result.games
    );
    Ok(())
}

fn status(run_dir: &Path) -> io::Result<()> {
    let run = load_run(run_dir)?;
    println!("run_dir={}", absolute(run_dir)?.display());
    println!("fresh_seed={INITIAL_SEED:#018x}");
    println!("games={}", run.games);
    println!("training_steps={}", run.model.training_steps);
    println!("champion_steps={}", run.champion.training_steps);
    println!("promotions={}", run.promotions);
    println!("recoveries={}", run.recoveries);
    println!("replay_positions={}", run.replay.len());
    Ok(())
}

fn recover(run_dir: &Path) -> io::Result<()> {
    let mut run = load_run(run_dir)?;
    if run.model.training_steps == run.champion.training_steps {
        println!(
            "latest already equals champion step {}",
            run.champion.training_steps
        );
        return Ok(());
    }
    let discarded_step = run.model.training_steps;
    run.model = run.champion.clone();
    run.optimizer = Adam::new();
    run.recoveries += 1;
    save_run(run_dir, &run, 0.0, None, true)?;
    println!(
        "recovered latest from step {discarded_step} to validated champion step {}; Adam reset, replay retained",
        run.model.training_steps
    );
    Ok(())
}

fn publish(run_dir: &Path) -> io::Result<()> {
    publish_to(run_dir, Path::new("crates/agent-fajita/model/fajita.bin"))
}

fn publish_to(run_dir: &Path, destination: &Path) -> io::Result<()> {
    let state_path = run_dir.join("state.txt");
    let state = fs::read_to_string(&state_path)?;
    if !state
        .lines()
        .any(|line| line == "purity=rules-only-fresh-seed")
    {
        return Err(io::Error::other(
            "run is missing Fajita's rules-only fresh-seed purity marker",
        ));
    }

    let read = |name: &str| {
        state
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .and_then(|value| value.parse::<u64>().ok())
    };
    let promotions = read("promotions")
        .ok_or_else(|| io::Error::other("run metadata is missing a valid promotions count"))?;
    if promotions == 0 {
        return Err(io::Error::other(
            "run has no validated champion promotion to publish",
        ));
    }

    let expected_champion_steps = read("champion_steps")
        .ok_or_else(|| io::Error::other("run metadata is missing champion_steps"))?;
    let source = run_dir.join("champion.bin");
    let champion = Model::load(&source)?;
    if champion.training_steps != expected_champion_steps {
        return Err(io::Error::other(format!(
            "champion checkpoint step {} does not match run metadata step {expected_champion_steps}",
            champion.training_steps
        )));
    }
    let latest = Model::load(run_dir.join("latest.bin"))?;
    let Some(parent) = destination.parent() else {
        return Err(io::Error::other("invalid publication path"));
    };
    fs::create_dir_all(parent)?;
    atomic_write(destination, &champion.to_bytes())?;
    println!(
        "Published validated Fajita champion step {} from {} to {} (latest training step {}, promotions {})",
        champion.training_steps,
        source.display(),
        destination.display(),
        latest.training_steps,
        promotions,
    );
    println!("Rebuild WASM to load the new checkpoint.");
    Ok(())
}

fn load_run(run_dir: &Path) -> io::Result<RunState> {
    let meta = load_meta(&run_dir.join("state.txt"))?;
    let latest_path = run_dir.join("latest.bin");
    let champion_path = run_dir.join("champion.bin");
    let optimizer_path = run_dir.join("optimizer.bin");
    let replay_path = run_dir.join("replay.bin");
    let model = if latest_path.exists() {
        Model::load(latest_path)?
    } else {
        Model::seeded(INITIAL_SEED)
    };
    let champion = if champion_path.exists() {
        Model::load(champion_path)?
    } else {
        model.clone()
    };
    let optimizer = if optimizer_path.exists() {
        Adam::from_bytes(&fs::read(optimizer_path)?)?
    } else {
        Adam::new()
    };
    let replay = if replay_path.exists() {
        load_replay(&replay_path)?
    } else {
        ReplayBuffer::new()
    };
    Ok(RunState {
        model,
        champion,
        optimizer,
        replay,
        games: meta.games,
        promotions: meta.promotions,
        recoveries: meta.recoveries,
        rng: Rng::new(meta.rng),
    })
}

struct Metadata {
    games: u64,
    promotions: u64,
    recoveries: u64,
    rng: u64,
}

fn load_meta(path: &Path) -> io::Result<Metadata> {
    if !path.exists() {
        return Ok(Metadata {
            games: 0,
            promotions: 0,
            recoveries: 0,
            rng: 0xFA71_2026_5E1F_0001,
        });
    }
    let text = fs::read_to_string(path)?;
    let read = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    };
    Ok(Metadata {
        games: read("games"),
        promotions: read("promotions"),
        recoveries: read("recoveries"),
        rng: read("rng"),
    })
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
    atomic_write(&run_dir.join("optimizer.bin"), &run.optimizer.to_bytes())?;
    if save_replay {
        save_replay_file(&run_dir.join("replay.bin"), &run.replay)?;
    }
    let metadata = format!(
        "purity=rules-only-fresh-seed\ngames={}\npromotions={}\nrecoveries={}\nrng={}\ntraining_steps={}\nchampion_steps={}\nreplay_positions={}\nlast_loss={loss}\nlast_arena={}\nupdated_unix={}\n",
        run.games,
        run.promotions,
        run.recoveries,
        run.rng.0,
        run.model.training_steps,
        run.champion.training_steps,
        run.replay.len(),
        arena.map_or_else(
            || "not-run".to_owned(),
            |result| format!(
                "score:{:.6},lower95:{:.6},games:{}",
                result.score, result.lower_95, result.games
            )
        ),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    atomic_write(&run_dir.join("state.txt"), metadata.as_bytes())
}

fn archive_champion(run_dir: &Path, run: &RunState) -> io::Result<()> {
    let league = run_dir.join("league");
    fs::create_dir_all(&league)?;
    atomic_write(
        &league.join(format!(
            "champion-{:04}-step-{}.bin",
            run.promotions, run.champion.training_steps
        )),
        &run.champion.to_bytes(),
    )
}

fn append_arena(run_dir: &Path, run: &RunState, result: ArenaResult) -> io::Result<()> {
    let path = run_dir.join("arena.csv");
    let new_file = !path.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if new_file {
        writeln!(
            file,
            "games,training_steps,promotions,score,lower95,arena_games"
        )?;
    }
    writeln!(
        file,
        "{},{},{},{:.6},{:.6},{}",
        run.games,
        run.model.training_steps,
        run.promotions,
        result.score,
        result.lower_95,
        result.games
    )
}

fn save_replay_file(path: &Path, replay: &ReplayBuffer) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    let mut writer = BufWriter::new(fs::File::create(&temporary)?);
    writer.write_all(REPLAY_MAGIC)?;
    writer.write_all(&(replay.len() as u64).to_le_bytes())?;
    for sample in replay.chronological() {
        writer.write_all(&sample.input)?;
        writer.write_all(
            &u16::try_from(sample.policy.len())
                .map_err(|_| invalid("replay policy exceeds u16"))?
                .to_le_bytes(),
        )?;
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
    if &magic != REPLAY_MAGIC {
        return Err(invalid("invalid Fajita replay"));
    }
    let count = read_u64(&mut reader)?;
    if count > MAX_REPLAY as u64 {
        return Err(invalid("Fajita replay exceeds configured capacity"));
    }
    let mut replay = ReplayBuffer::new();
    for _ in 0..count {
        let mut input = [0; PACKED_INPUT_SIZE];
        reader.read_exact(&mut input)?;
        let policy_len = usize::from(read_u16(&mut reader)?);
        if policy_len == 0 || policy_len > ACTION_SIZE {
            return Err(invalid("invalid Fajita replay policy length"));
        }
        let mut policy = Vec::with_capacity(policy_len);
        let mut seen = [false; ACTION_SIZE];
        for _ in 0..policy_len {
            let index = read_u16(&mut reader)?;
            let weight = read_u16(&mut reader)?;
            if usize::from(index) >= ACTION_SIZE || weight == 0 || seen[usize::from(index)] {
                return Err(invalid("invalid sparse Fajita replay policy"));
            }
            seen[usize::from(index)] = true;
            policy.push((index, weight));
        }
        let mut value = [0];
        reader.read_exact(&mut value)?;
        let value = i8::from_le_bytes(value);
        if !(-1..=1).contains(&value) {
            return Err(invalid("invalid Fajita replay outcome"));
        }
        replay.push(CompactSample {
            input,
            policy: policy.into_boxed_slice(),
            value,
        });
    }
    let mut trailing = [0];
    if reader.read(&mut trailing)? != 0 {
        return Err(invalid("Fajita replay contains trailing bytes"));
    }
    Ok(replay)
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

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
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

    #[test]
    fn compact_inputs_round_trip() {
        for seed in 0..16 {
            let input = encode_state(&initial_state(seed));
            let packed = compact_input(&input);
            for (index, expected) in input.into_iter().enumerate() {
                assert_eq!(f32::from(packed_feature(&packed, index)) * 0.5, expected);
            }
        }
    }

    #[test]
    fn replay_ring_keeps_newest_entries() {
        let mut replay = ReplayBuffer::with_capacity(3);
        for marker in 0..5_u16 {
            replay.push(CompactSample {
                input: [0; PACKED_INPUT_SIZE],
                policy: vec![(marker, u16::MAX)].into_boxed_slice(),
                value: 0,
            });
        }
        let markers = replay
            .chronological()
            .map(|sample| sample.policy[0].0)
            .collect::<Vec<_>>();
        assert_eq!(markers, [2, 3, 4]);
    }

    #[test]
    fn adaptive_budget_covers_wide_roots() {
        assert_eq!(adaptive_simulations(32, 290), 870);
        assert_eq!(adaptive_simulations(256, 100), 300);
        assert_eq!(adaptive_simulations(2_000, 290), 2_000);
    }

    #[test]
    fn mature_models_use_the_lower_learning_rate() {
        assert_eq!(learning_rate(MATURE_STEP - 1), INITIAL_LEARNING_RATE);
        assert_eq!(learning_rate(MATURE_STEP), MATURE_LEARNING_RATE);
    }

    #[test]
    fn publish_exports_the_validated_champion_instead_of_latest() {
        let root = test_directory("publish-champion");
        let run_dir = root.join("run");
        let destination = root.join("published/fajita.bin");
        fs::create_dir_all(&run_dir).unwrap();

        let mut champion = Model::seeded(INITIAL_SEED);
        champion.training_steps = 560_000;
        champion.save(run_dir.join("champion.bin")).unwrap();
        let mut latest = Model::seeded(INITIAL_SEED);
        latest.training_steps = 560_100;
        latest.save(run_dir.join("latest.bin")).unwrap();
        fs::write(
            run_dir.join("state.txt"),
            "purity=rules-only-fresh-seed\npromotions=41\nchampion_steps=560000\n",
        )
        .unwrap();

        publish_to(&run_dir, &destination).unwrap();

        let published = Model::load(destination).unwrap();
        assert_eq!(published.training_steps, champion.training_steps);
        assert_eq!(published.to_bytes(), champion.to_bytes());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn publish_rejects_an_unpromoted_run() {
        let root = test_directory("publish-unpromoted");
        let run_dir = root.join("run");
        fs::create_dir_all(&run_dir).unwrap();
        let model = Model::seeded(INITIAL_SEED);
        model.save(run_dir.join("champion.bin")).unwrap();
        model.save(run_dir.join("latest.bin")).unwrap();
        fs::write(
            run_dir.join("state.txt"),
            "purity=rules-only-fresh-seed\npromotions=0\nchampion_steps=0\n",
        )
        .unwrap();

        let error = publish_to(&run_dir, &root.join("published/fajita.bin")).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("no validated champion promotion")
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn test_directory(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!(
            "fajita-train-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
