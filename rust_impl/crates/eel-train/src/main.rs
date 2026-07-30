use agent_avocado::AvocadoAnalyzer;
use agent_cherry::CherryAnalyzer;
use agent_eel::{
    ACTION_SIZE, EelAnalyzer, INPUT_SIZE, Model, Search, action_index, encode_state, state_key,
    training::{Adam, Sample},
};
use snipe_core::{
    Action, Analyzer, Animal, Card, CardMultiset, Evaluation, Player, Rank, State, StepDirection,
};
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

const DEFAULT_RUN_DIR: &str = "training/eel-main";
const DEFAULT_HOURS: f64 = 8.0;
const DEFAULT_SIMULATIONS: usize = 128;
const INITIAL_MODEL_SEED: u64 = 0xEE17_5EA5_2026_0001;
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
const MATURE_STEP: u64 = 200_000;
const REGRESSION_SCORE: f32 = 0.45;
const PACKED_INPUT_SIZE: usize = INPUT_SIZE.div_ceil(4);
const REPLAY_MAGIC: &[u8; 8] = b"EELRPL01";

fn main() -> ExitCode {
    match run() {
        Ok(success) if success => ExitCode::SUCCESS,
        Ok(_) => ExitCode::from(2),
        Err(error) => {
            eprintln!("eel-train: {error}");
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
        "tournament" => tournament_command(&run_dir, &arguments),
        "help" | "--help" | "-h" => {
            help();
            Ok(true)
        }
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command {other:?}; use `eel-train help`"),
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
        "Eel rules-only self-play trainer\n\
         \n\
         train|nightly [--run-dir PATH] [--hours N] [--simulations N] [--workers N]\n\
         status        [--run-dir PATH]\n\
         evaluate      [--run-dir PATH] [--pairs N] [--simulations N]\n\
         recover       [--run-dir PATH]\n\
         tournament    [--run-dir PATH] --opponent cherry|avocado --log-dir PATH\n\
                       [--checkpoint latest|champion|PATH] [--pairs 10]\n\
                       [--eel-ms 5000] [--older-ms 10000] [--seed-start 0]\n\
         \n\
         A new run always starts from deterministic fresh weights. Training consumes only\n\
         legal self-play positions, MCTS visit policies, and final game outcomes."
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
        "Eel self-play resumed: games={}, steps={}, replay={}, base simulations/action={}, workers={}",
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
            .expect("Eel self-play selected a legal action");
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
    base.max(branching_factor.saturating_mul(2).min(1_024))
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
                    let seed = 0xEE1A_0000_0000_0000 ^ round.rotate_left(19) ^ pair as u64;
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
    println!("fresh_seed={INITIAL_MODEL_SEED:#018x}");
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

fn load_run(run_dir: &Path) -> io::Result<RunState> {
    let meta = load_meta(&run_dir.join("state.txt"))?;
    let latest_path = run_dir.join("latest.bin");
    let champion_path = run_dir.join("champion.bin");
    let optimizer_path = run_dir.join("optimizer.bin");
    let replay_path = run_dir.join("replay.bin");
    let model = if latest_path.exists() {
        Model::load(latest_path)?
    } else {
        Model::seeded(INITIAL_MODEL_SEED)
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
            rng: 0xEE17_2026_5E1F_0001,
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
        return Err(invalid("invalid Eel replay"));
    }
    let count = read_u64(&mut reader)?;
    if count > MAX_REPLAY as u64 {
        return Err(invalid("Eel replay exceeds configured capacity"));
    }
    let mut replay = ReplayBuffer::new();
    for _ in 0..count {
        let mut input = [0; PACKED_INPUT_SIZE];
        reader.read_exact(&mut input)?;
        let policy_len = usize::from(read_u16(&mut reader)?);
        if policy_len == 0 || policy_len > ACTION_SIZE {
            return Err(invalid("invalid Eel replay policy length"));
        }
        let mut policy = Vec::with_capacity(policy_len);
        let mut seen = [false; ACTION_SIZE];
        for _ in 0..policy_len {
            let index = read_u16(&mut reader)?;
            let weight = read_u16(&mut reader)?;
            if usize::from(index) >= ACTION_SIZE || weight == 0 || seen[usize::from(index)] {
                return Err(invalid("invalid sparse Eel replay policy"));
            }
            seen[usize::from(index)] = true;
            policy.push((index, weight));
        }
        let mut value = [0];
        reader.read_exact(&mut value)?;
        let value = i8::from_le_bytes(value);
        if !(-1..=1).contains(&value) {
            return Err(invalid("invalid Eel replay outcome"));
        }
        replay.push(CompactSample {
            input,
            policy: policy.into_boxed_slice(),
            value,
        });
    }
    let mut trailing = [0];
    if reader.read(&mut trailing)? != 0 {
        return Err(invalid("Eel replay contains trailing bytes"));
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

#[derive(Clone, Copy)]
enum Opponent {
    Cherry,
    Avocado,
}

impl Opponent {
    fn parse(value: &str) -> io::Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "cherry" => Ok(Self::Cherry),
            "avocado" => Ok(Self::Avocado),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "--opponent must be cherry or avocado",
            )),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cherry => "Cherry",
            Self::Avocado => "Avocado",
        }
    }

    fn create(self) -> ExternalAgent {
        match self {
            Self::Cherry => ExternalAgent::Cherry(CherryAnalyzer::new()),
            Self::Avocado => ExternalAgent::Avocado(AvocadoAnalyzer::new()),
        }
    }
}

enum ExternalAgent {
    Eel(EelAnalyzer),
    Cherry(CherryAnalyzer),
    Avocado(AvocadoAnalyzer),
}

impl ExternalAgent {
    fn set_state(&mut self, state: State) {
        match self {
            Self::Eel(agent) => agent.set_state(state),
            Self::Cherry(agent) => agent.set_state(state),
            Self::Avocado(agent) => agent.set_state(state),
        }
    }

    fn tick(&mut self) {
        match self {
            Self::Eel(agent) => agent.think_for_one_tick(),
            Self::Cherry(agent) => agent.think_for_one_tick(),
            Self::Avocado(agent) => agent.think_for_one_tick(),
        }
    }

    fn evaluation(&self) -> Evaluation {
        match self {
            Self::Eel(agent) => agent.evaluation(),
            Self::Cherry(agent) => agent.evaluation(),
            Self::Avocado(agent) => agent.evaluation(),
        }
    }

    fn line(&self) -> Vec<Action> {
        let mut line = Vec::new();
        match self {
            Self::Eel(agent) => agent.write_optimal_lop(&mut line),
            Self::Cherry(agent) => agent.write_optimal_lop(&mut line),
            Self::Avocado(agent) => agent.write_optimal_lop(&mut line),
        }
        line
    }
}

fn tournament_command(run_dir: &Path, arguments: &[String]) -> io::Result<bool> {
    let opponent = Opponent::parse(
        option(arguments, "--opponent")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing --opponent"))?,
    )?;
    let log_dir = PathBuf::from(
        option(arguments, "--log-dir")
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing --log-dir"))?,
    );
    let pairs = parse_option(arguments, "--pairs", 10_u64)?;
    let eel_ms = parse_option(arguments, "--eel-ms", 5_000_u64)?;
    let older_ms = parse_option(arguments, "--older-ms", 10_000_u64)?;
    let seed_start = parse_option(arguments, "--seed-start", 0_u64)?;
    if pairs == 0 || eel_ms == 0 || older_ms == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "pairs and time limits must be positive",
        ));
    }
    let checkpoint = option(arguments, "--checkpoint").unwrap_or("latest");
    let checkpoint_path = match checkpoint {
        "latest" => run_dir.join("latest.bin"),
        "champion" => run_dir.join("champion.bin"),
        path => PathBuf::from(path),
    };
    let model = Model::load(&checkpoint_path)?;
    run_external_tournament(
        &model,
        opponent,
        pairs,
        Duration::from_millis(eel_ms),
        Duration::from_millis(older_ms),
        seed_start,
        &log_dir,
        &checkpoint_path,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_external_tournament(
    model: &Model,
    opponent: Opponent,
    pairs: u64,
    eel_time: Duration,
    older_time: Duration,
    seed_start: u64,
    log_dir: &Path,
    checkpoint: &Path,
) -> io::Result<bool> {
    fs::create_dir_all(log_dir)?;
    let started = Instant::now();
    let mut wins = 0;
    let mut completed = 0;
    println!(
        "Eel step {} ({:.3}s/ply) vs {} ({:.3}s/ply), {} paired seeds",
        model.training_steps,
        eel_time.as_secs_f64(),
        opponent.label(),
        older_time.as_secs_f64(),
        pairs
    );
    for pair in 0..pairs {
        let seed = seed_start.wrapping_add(pair);
        for eel_side in [Player::Alpha, Player::Beta] {
            let filename = format!(
                "seed-{seed:016x}-eel-{}.shgh",
                if eel_side == Player::Alpha {
                    "alpha"
                } else {
                    "beta"
                }
            );
            let path = log_dir.join(filename);
            let report = play_external_game(model, opponent, seed, eel_side, eel_time, older_time)?;
            let eel_won = report.winner == Some(eel_side);
            wins += usize::from(eel_won);
            completed += 1;
            append_tournament_summary(
                log_dir,
                checkpoint,
                model.training_steps,
                opponent,
                seed,
                eel_side,
                &report,
                eel_won,
            )?;
            let text = report.history.finish(
                report.winner,
                if eel_won {
                    "Eel won"
                } else if report.winner.is_some() {
                    "older agent won"
                } else {
                    "draw"
                },
            );
            atomic_write(&path, text.as_bytes())?;
            println!(
                "game {completed:>2}/{} seed={seed} eel={eel_side:?} result={} plies={} ticks(eel={},older={}) total={:.1}s log={}",
                pairs * 2,
                if eel_won {
                    "win"
                } else if report.winner.is_some() {
                    "loss"
                } else {
                    "draw"
                },
                report.plies,
                report.eel_ticks,
                report.older_ticks,
                started.elapsed().as_secs_f64(),
                path.display()
            );
            if !eel_won {
                println!(
                    "tournament aborted: Eel can no longer finish 20/20 after game {completed}"
                );
                return Ok(false);
            }
        }
    }
    println!(
        "tournament complete: Eel {wins}/{} against {}",
        pairs * 2,
        opponent.label()
    );
    Ok(wins == (pairs * 2) as usize)
}

struct ExternalGameReport {
    winner: Option<Player>,
    plies: u32,
    eel_ticks: u64,
    older_ticks: u64,
    history: History,
}

fn play_external_game(
    model: &Model,
    opponent: Opponent,
    seed: u64,
    eel_side: Player,
    eel_time: Duration,
    older_time: Duration,
) -> io::Result<ExternalGameReport> {
    let mut state = initial_state(seed);
    let (alpha_label, beta_label) = if eel_side == Player::Alpha {
        ("Eel", opponent.label())
    } else {
        (opponent.label(), "Eel")
    };
    let mut history = History::new(
        &state,
        seed,
        alpha_label,
        beta_label,
        if eel_side == Player::Alpha {
            eel_time
        } else {
            older_time
        },
        if eel_side == Player::Beta {
            eel_time
        } else {
            older_time
        },
    );
    let mut eel = ExternalAgent::Eel(EelAnalyzer::with_model(model.clone()));
    let mut older = opponent.create();
    let mut seen = HashSet::new();
    let mut eel_ticks = 0;
    let mut older_ticks = 0;
    for ply in 0..MAX_ATOMIC_ACTIONS as u32 {
        if let Some(winner) = state.winner() {
            return Ok(ExternalGameReport {
                winner: Some(winner),
                plies: ply,
                eel_ticks,
                older_ticks,
                history,
            });
        }
        if !seen.insert(state_key(&state)) {
            return Ok(ExternalGameReport {
                winner: None,
                plies: ply,
                eel_ticks,
                older_ticks,
                history,
            });
        }
        let player = state.active_player;
        let eel_turn = player == eel_side;
        let (agent, budget, ticks) = if eel_turn {
            (&mut eel, eel_time, &mut eel_ticks)
        } else {
            (&mut older, older_time, &mut older_ticks)
        };
        agent.set_state(state.clone());
        let thinking_started = Instant::now();
        loop {
            agent.tick();
            *ticks += 1;
            if thinking_started.elapsed() >= budget {
                break;
            }
        }
        let line = agent.line();
        if line.is_empty() {
            return Err(invalid(format!(
                "{} returned an empty line at evaluation {:?}",
                if eel_turn { "Eel" } else { opponent.label() },
                agent.evaluation()
            )));
        }
        let turn_start = state.clone();
        let mut actions = Vec::with_capacity(2);
        for action in line {
            actions.push(action);
            state = state.apply(action).map_err(|error| {
                invalid(format!(
                    "{} returned illegal action {action:?}: {error:?}",
                    if eel_turn { "Eel" } else { opponent.label() }
                ))
            })?;
            if state.active_player != player || state.winner().is_some() {
                break;
            }
        }
        if state.active_player == player && state.winner().is_none() {
            return Err(invalid(format!(
                "{} returned an incomplete ply",
                if eel_turn { "Eel" } else { opponent.label() }
            )));
        }
        history.record_turn(ply + 1, &turn_start, &actions)?;
    }
    Ok(ExternalGameReport {
        winner: state.winner(),
        plies: MAX_ATOMIC_ACTIONS as u32,
        eel_ticks,
        older_ticks,
        history,
    })
}

#[allow(clippy::too_many_arguments)]
fn append_tournament_summary(
    log_dir: &Path,
    checkpoint: &Path,
    training_steps: u64,
    opponent: Opponent,
    seed: u64,
    eel_side: Player,
    report: &ExternalGameReport,
    eel_won: bool,
) -> io::Result<()> {
    let path = log_dir.join("summary.csv");
    let new_file = !path.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if new_file {
        writeln!(
            file,
            "checkpoint,training_steps,opponent,seed,eel_side,result,plies,eel_ticks,older_ticks"
        )?;
    }
    writeln!(
        file,
        "{},{},{},{},{:?},{},{},{},{}",
        checkpoint.display(),
        training_steps,
        opponent.label(),
        seed,
        eel_side,
        if eel_won {
            "win"
        } else if report.winner.is_some() {
            "loss"
        } else {
            "draw"
        },
        report.plies,
        report.eel_ticks,
        report.older_ticks
    )
}

struct History {
    lines: Vec<String>,
}

impl History {
    fn new(
        state: &State,
        seed: u64,
        alpha: &str,
        beta: &str,
        alpha_time: Duration,
        beta_time: Duration,
    ) -> Self {
        Self {
            lines: vec![
                format!(
                    "// Beta: {beta} ({} seconds per ply)",
                    beta_time.as_secs_f64()
                ),
                format!(
                    "// Alpha: {alpha} ({} seconds per ply)",
                    alpha_time.as_secs_f64()
                ),
                format!("// Seed: {seed}"),
                String::new(),
                format!(
                    "0b. ={}; {}; {}; {}",
                    format_cards(state.reserves, Player::Beta),
                    format_cards(state.r6, Player::Beta),
                    format_cards(state.r5, Player::Beta),
                    format_cards(state.r4, Player::Beta)
                ),
                format!(
                    "0a. ={}; {}; {}; {}",
                    format_cards(state.reserves, Player::Alpha),
                    format_cards(state.r1, Player::Alpha),
                    format_cards(state.r2, Player::Alpha),
                    format_cards(state.r3, Player::Alpha)
                ),
            ],
        }
    }

    fn record_turn(&mut self, index: u32, state: &State, actions: &[Action]) -> io::Result<()> {
        let player = state.active_player;
        let mut position = state.clone();
        let mut formatted = Vec::with_capacity(actions.len());
        for &action in actions {
            formatted.push(format_action(&position, action));
            position = position
                .apply(action)
                .map_err(|error| invalid(format!("cannot log action: {error:?}")))?;
        }
        if let Some(last) = formatted.last_mut()
            && let Some(winner) = position.winner()
        {
            last.push_str(if winner == Player::Alpha {
                "+#0"
            } else {
                "-#0"
            });
        }
        self.lines.push(format!(
            "{index}{}. {}",
            if player == Player::Alpha { 'a' } else { 'b' },
            formatted.join(", ")
        ));
        Ok(())
    }

    fn finish(mut self, winner: Option<Player>, result: &str) -> String {
        self.lines.push(String::new());
        self.lines.push(format!("// Result: {result}"));
        self.lines.push(format!("// Winner: {winner:?}"));
        format!("{}\n", self.lines.join("\n"))
    }
}

fn format_action(state: &State, action: Action) -> String {
    match action {
        Action::AnimalStep(step) => {
            let destination = cards_at(state, step.destination);
            let capture = step.actor.would_activate_triplet_by_entering(destination)
                && animal_count(destination) > 0;
            format!(
                "{} {}{}{}",
                animal_name(step.actor),
                if step.direction == StepDirection::Retreat {
                    "*"
                } else {
                    ""
                },
                rank_number(step.destination),
                if capture { "x" } else { "" }
            )
        }
        Action::SnipeStep(step) => {
            let source = RANKS
                .into_iter()
                .find(|&rank| cards_at(state, rank).count(Card::Snipe, state.active_player) != 0)
                .expect("live player has a snipe");
            let retreating = match state.active_player {
                Player::Alpha => rank_number(step.destination) < rank_number(source),
                Player::Beta => rank_number(step.destination) > rank_number(source),
            };
            format!(
                "{} {}{}",
                player_name(state.active_player),
                if retreating { "*" } else { "" },
                rank_number(step.destination)
            )
        }
        Action::Drop(drop) => {
            format!(
                "{} &{}",
                animal_name(drop.actor),
                rank_number(drop.destination)
            )
        }
    }
}

fn format_cards(cards: CardMultiset, owner: Player) -> String {
    let mut names = Vec::new();
    for animal in ANIMALS {
        for _ in 0..cards.count(Card::Animal(animal), owner) {
            names.push(animal_name(animal));
        }
    }
    if cards.count(Card::Snipe, owner) > 0 {
        names.push(player_name(owner));
    }
    names.join(" ")
}

fn animal_count(cards: CardMultiset) -> u32 {
    ANIMALS
        .into_iter()
        .map(|animal| {
            u32::from(cards.count(Card::Animal(animal), Player::Alpha))
                + u32::from(cards.count(Card::Animal(animal), Player::Beta))
        })
        .sum()
}

fn cards_at(state: &State, rank: Rank) -> CardMultiset {
    match rank {
        Rank::R1 => state.r1,
        Rank::R2 => state.r2,
        Rank::R3 => state.r3,
        Rank::R4 => state.r4,
        Rank::R5 => state.r5,
        Rank::R6 => state.r6,
    }
}

fn rank_number(rank: Rank) -> u8 {
    match rank {
        Rank::R1 => 1,
        Rank::R2 => 2,
        Rank::R3 => 3,
        Rank::R4 => 4,
        Rank::R5 => 5,
        Rank::R6 => 6,
    }
}

fn player_name(player: Player) -> &'static str {
    match player {
        Player::Alpha => "Alpha",
        Player::Beta => "Beta",
    }
}

fn animal_name(animal: Animal) -> &'static str {
    match animal {
        Animal::Mouse => "Rat",
        Animal::Ox => "Ox",
        Animal::Tiger => "Tiger",
        Animal::Rabbit => "Rabbit",
        Animal::Dragon => "Dragon",
        Animal::Snake => "Snake",
        Animal::Horse => "Horse",
        Animal::Ram => "Ram",
        Animal::Monkey => "Monkey",
        Animal::Rooster => "Rooster",
        Animal::Dog => "Dog",
        Animal::Boar => "Boar",
        Animal::Fish => "Fish",
        Animal::Elephant => "Elephant",
        Animal::Squid => "Squid",
        Animal::Frog => "Frog",
    }
}

const ANIMALS: [Animal; 16] = [
    Animal::Mouse,
    Animal::Ox,
    Animal::Tiger,
    Animal::Rabbit,
    Animal::Dragon,
    Animal::Snake,
    Animal::Horse,
    Animal::Ram,
    Animal::Monkey,
    Animal::Rooster,
    Animal::Dog,
    Animal::Boar,
    Animal::Fish,
    Animal::Elephant,
    Animal::Squid,
    Animal::Frog,
];

const RANKS: [Rank; 6] = [Rank::R1, Rank::R2, Rank::R3, Rank::R4, Rank::R5, Rank::R6];

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
        assert_eq!(adaptive_simulations(32, 290), 580);
        assert_eq!(adaptive_simulations(256, 100), 256);
        assert_eq!(adaptive_simulations(2_000, 290), 2_000);
    }

    #[test]
    fn mature_models_use_the_lower_learning_rate() {
        assert_eq!(learning_rate(MATURE_STEP - 1), INITIAL_LEARNING_RATE);
        assert_eq!(learning_rate(MATURE_STEP), MATURE_LEARNING_RATE);
    }
}
