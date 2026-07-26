use agent_cherry::{
    ACTION_SIZE, INPUT_SIZE, Model, Search, action_index, encode_state,
    training::{Adam, Sample},
};
use snipe_core::{Action, Player, initial_state};
use std::{
    collections::HashSet,
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::ExitCode,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const DEFAULT_RUN_DIR: &str = "training/cherry-main";
const DEFAULT_HOURS: f64 = 8.0;
const DEFAULT_SIMULATIONS: usize = 12;
const MAX_REPLAY: usize = 12_000;
const MAX_ATOMIC_ACTIONS: usize = 256;
const BATCH_SIZE: usize = 32;
const BATCHES_PER_GAME: usize = 6;

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
        "train" | "nightly" => {
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
         train|nightly [--run-dir PATH] [--hours N] [--simulations N] [--workers N]\n\
         status        [--run-dir PATH]\n\
         evaluate      [--run-dir PATH] [--simulations N]\n\
         audit         [--run-dir PATH] [--simulations N] [--pairs N]\n\
         publish       [--run-dir PATH]\n\
         \n\
         Weights checkpoint after every completed game and replay every ten.\n\
         Stop with Ctrl-C and rerun the same command to resume."
    );
}

struct RunState {
    model: Model,
    champion: Model,
    optimizer: Adam,
    replay: Vec<Sample>,
    games: u64,
    promotions: u64,
    rng: Rng,
}

fn default_workers() -> usize {
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .saturating_sub(1)
        .max(1)
}

fn train(run_dir: &Path, duration: Duration, simulations: usize, workers: usize) -> io::Result<()> {
    fs::create_dir_all(run_dir)?;
    let mut run = load_run(run_dir)?;
    let started = Instant::now();
    println!(
        "Cherry training resumed: games={}, steps={}, replay={}, simulations/action={}, workers={}",
        run.games,
        run.model.training_steps,
        run.replay.len(),
        simulations,
        workers
    );
    println!("Run directory: {}", absolute(run_dir)?.display());
    println!("Stop safely with Ctrl-C; completed games are already durable.");

    while started.elapsed() < duration {
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
            for (mut samples, winner, actions) in receiver {
                run.replay.append(&mut samples);
                if run.replay.len() > MAX_REPLAY {
                    run.replay.drain(..run.replay.len() - MAX_REPLAY);
                }
                run.games += 1;

                let mut loss = 0.0;
                if !run.replay.is_empty() {
                    for _ in 0..BATCHES_PER_GAME {
                        let batch = random_batch(&run.replay, BATCH_SIZE, &mut run.rng);
                        loss += run.model.train_batch(&batch, &mut run.optimizer, 0.0005);
                    }
                    loss /= BATCHES_PER_GAME as f32;
                }

                let mut arena_result = None;
                if run.games % 50 == 0 {
                    let result =
                        arena(&run.model, &run.champion, simulations.max(8), run.games, 32);
                    if result.lower_bound > 0.5
                        && passes_league_guard(
                            run_dir,
                            &run.model,
                            &run.champion,
                            simulations.max(8),
                            run.games,
                        )?
                    {
                        run.champion = run.model.clone();
                        run.promotions += 1;
                        archive_champion(run_dir, &run)?;
                    }
                    append_arena_report(run_dir, &run, result)?;
                    arena_result = Some(result);
                }
                save_run(
                    run_dir,
                    &run,
                    loss,
                    arena_result,
                    run.games < 10 || run.games % 10 == 0,
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
                        " arena={:.3} lower95={:.3}",
                        result.score, result.lower_bound
                    )),
                );
            }
            Ok::<(), io::Error>(())
        })?;
    }
    println!(
        "Training window complete: games={}, steps={}, promotions={}",
        run.games, run.model.training_steps, run.promotions
    );
    Ok(())
}

fn self_play(
    model: &Model,
    seed: u64,
    simulations: usize,
    rng: &mut Rng,
) -> (Vec<Sample>, Option<Player>, usize) {
    let mut state = initial_state(seed);
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    let mut winner = None;
    for action_number in 0..MAX_ATOMIC_ACTIONS {
        if let Some(found) = state.winner() {
            winner = Some(found);
            break;
        }
        let fingerprint = format!("{state:?}");
        if !seen.insert(fingerprint) {
            break;
        }
        let mut search = Search::new(state.clone(), model);
        search.simulate_n(model, simulations);
        let policy = search.policy(if action_number < 30 { 1.0 } else { 0.05 });
        if policy.is_empty() {
            winner = state.winner();
            break;
        }
        let mut target = vec![0.0; ACTION_SIZE];
        for &(action, probability) in &policy {
            target[action_index(&state, action)] = probability;
        }
        records.push((encode_state(&state), target, state.active_player));
        let action = exploratory_choice(&policy, rng);
        state = state
            .apply(action)
            .expect("MCTS policy contains legal actions");
    }
    winner = winner.or_else(|| state.winner());
    let samples = records
        .into_iter()
        .map(|(input, policy, player)| Sample {
            input,
            policy,
            value: winner.map_or(0.0, |won| if won == player { 1.0 } else { -1.0 }),
        })
        .collect::<Vec<_>>();
    (samples, winner, seen.len())
}

fn exploratory_choice(policy: &[(Action, f32)], rng: &mut Rng) -> Action {
    if rng.unit() < 0.08 {
        return policy[(rng.next_u64() as usize) % policy.len()].0;
    }
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

fn random_batch(replay: &[Sample], size: usize, rng: &mut Rng) -> Vec<Sample> {
    (0..size.min(replay.len()))
        .map(|_| {
            let sample = &replay[(rng.next_u64() as usize) % replay.len()];
            Sample {
                input: sample.input.clone(),
                policy: sample.policy.clone(),
                value: sample.value,
            }
        })
        .collect()
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
) -> ArenaResult {
    let mut paired_scores = Vec::with_capacity(pairs);
    for pair in 0..pairs {
        let seed = 0xA2E1_0000_0000_0000 ^ round.rotate_left(17) ^ pair as u64;
        let alpha_score = play_match(candidate, incumbent, seed, simulations);
        let beta_score = 1.0 - play_match(incumbent, candidate, seed, simulations);
        paired_scores.push((alpha_score + beta_score) * 0.5);
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
        lower_bound: (score - 1.645 * standard_error).clamp(0.0, 1.0),
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
    for _ in 0..MAX_ATOMIC_ACTIONS {
        if let Some(winner) = state.winner() {
            return if winner == Player::Alpha { 1.0 } else { 0.0 };
        }
        if !seen.insert(format!("{state:?}")) {
            return 0.5;
        }
        let (model, simulations) = if state.active_player == Player::Alpha {
            (alpha, alpha_simulations)
        } else {
            (beta, beta_simulations)
        };
        let mut search = Search::new(state.clone(), model);
        search.simulate_n(model, simulations);
        let Some((action, _)) = search.policy(0.0).into_iter().find(|(_, p)| *p > 0.0) else {
            return 0.5;
        };
        state = state.apply(action).expect("search returns legal action");
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
    let run = load_run(run_dir)?;
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
    let run = load_run(run_dir)?;
    let result = arena(&run.model, &run.champion, simulations, run.games + 1, pairs);
    println!(
        "latest vs staged champion: score={:.3}, lower95={:.3}, games={}, simulations/action={simulations}",
        result.score, result.lower_bound, result.games
    );
    Ok(())
}

fn publish(run_dir: &Path) -> io::Result<()> {
    let source = run_dir.join("latest.bin");
    let model = Model::load(&source)?;
    let destination = PathBuf::from("crates/agent-cherry/model/cherry.bin");
    let Some(parent) = destination.parent() else {
        return Err(io::Error::other("invalid publication path"));
    };
    fs::create_dir_all(parent)?;
    atomic_write(&destination, &model.to_bytes())?;
    println!(
        "Published Cherry step {} from {} to {}",
        model.training_steps,
        source.display(),
        destination.display()
    );
    println!("Rebuild WASM to load the new checkpoint.");
    Ok(())
}

fn status(run_dir: &Path) -> io::Result<()> {
    let run = load_run(run_dir)?;
    println!("run={}", absolute(run_dir)?.display());
    println!("games={}", run.games);
    println!("training_steps={}", run.model.training_steps);
    println!("replay_positions={}", run.replay.len());
    println!("promotions={}", run.promotions);
    println!(
        "latest={}",
        absolute(&run_dir.join("latest.bin"))?.display()
    );
    Ok(())
}

fn load_run(run_dir: &Path) -> io::Result<RunState> {
    let latest = run_dir.join("latest.bin");
    let model = if latest.exists() {
        Model::load(&latest)?
    } else {
        Model::seeded(0xC4E2_9917_D15C_A11E)
    };
    let champion_path = run_dir.join("champion.bin");
    let champion = if champion_path.exists() {
        Model::load(champion_path)?
    } else {
        model.clone()
    };
    let replay = if run_dir.join("replay.bin").exists() {
        load_replay(&run_dir.join("replay.bin"))?
    } else {
        Vec::new()
    };
    let optimizer = if run_dir.join("optimizer.bin").exists() {
        Adam::from_bytes(&fs::read(run_dir.join("optimizer.bin"))?)?
    } else {
        Adam::new()
    };
    let (games, promotions, rng_seed) = load_meta(&run_dir.join("state.txt"))?;
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
        atomic_write(&run_dir.join("replay.bin"), &replay_bytes(&run.replay))?;
        atomic_write(&run_dir.join("optimizer.bin"), &run.optimizer.to_bytes())?;
    }
    let metadata = format!(
        "games={}\npromotions={}\nrng={}\ntraining_steps={}\nreplay_positions={}\nlast_loss={loss}\nlast_arena={}\nupdated_unix={}\n",
        run.games,
        run.promotions,
        run.rng.0,
        run.model.training_steps,
        run.replay.len(),
        arena.map_or_else(
            || "not-run".to_owned(),
            |result| format!(
                "score:{:.6},lower95:{:.6},games:{}",
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
    let league = run_dir.join("league");
    fs::create_dir_all(&league)?;
    run.champion.save(league.join(format!(
        "champion-{:04}-step-{}.bin",
        run.promotions, run.champion.training_steps
    )))
}

fn append_arena_report(run_dir: &Path, run: &RunState, result: ArenaResult) -> io::Result<()> {
    use std::io::Write as _;

    let path = run_dir.join("arena.csv");
    let is_new = !path.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    if is_new {
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
) -> io::Result<bool> {
    let league = run_dir.join("league");
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
        candidate_score += arena(candidate, &anchor, simulations, arena_round, 8).score;
        incumbent_score += arena(incumbent, &anchor, simulations, arena_round, 8).score;
    }
    candidate_score /= checkpoints.len() as f32;
    incumbent_score /= checkpoints.len() as f32;
    println!(
        "league guard: candidate={candidate_score:.3}, incumbent={incumbent_score:.3}, checkpoints={}",
        checkpoints.len()
    );
    Ok(candidate_score + 0.02 >= incumbent_score)
}

fn load_meta(path: &Path) -> io::Result<(u64, u64, u64)> {
    if !path.exists() {
        return Ok((0, 0, 0x51A7_E5E5_C4E2_0001));
    }
    let text = fs::read_to_string(path)?;
    let get = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(&format!("{name}=")))
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0)
    };
    Ok((get("games"), get("promotions"), get("rng").max(1)))
}

fn replay_bytes(replay: &[Sample]) -> Vec<u8> {
    let floats_per_sample = INPUT_SIZE + ACTION_SIZE + 1;
    let mut bytes = Vec::with_capacity(16 + replay.len() * floats_per_sample * 4);
    bytes.extend_from_slice(b"CHREPLAY");
    bytes.extend_from_slice(&(replay.len() as u64).to_le_bytes());
    for sample in replay {
        for value in sample
            .input
            .iter()
            .chain(&sample.policy)
            .chain(std::iter::once(&sample.value))
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

fn load_replay(path: &Path) -> io::Result<Vec<Sample>> {
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if bytes.len() < 16 || &bytes[..8] != b"CHREPLAY" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid Cherry replay",
        ));
    }
    let count = u64::from_le_bytes(bytes[8..16].try_into().expect("checked")) as usize;
    let floats_per_sample = INPUT_SIZE + ACTION_SIZE + 1;
    if bytes.len() != 16 + count * floats_per_sample * 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "truncated Cherry replay",
        ));
    }
    let values = bytes[16..]
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
        .collect::<Vec<_>>();
    Ok(values
        .chunks_exact(floats_per_sample)
        .map(|sample| Sample {
            input: sample[..INPUT_SIZE].to_vec(),
            policy: sample[INPUT_SIZE..INPUT_SIZE + ACTION_SIZE].to_vec(),
            value: sample[floats_per_sample - 1],
        })
        .collect())
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
