# Snipe Hunt — clean Rust engine

The active implementation is intentionally small and dependency-directed:

- `crates/snipe-core` is the authoritative rules and public `Analyzer` contract.
- `crates/snipe-prng` owns reproducible seeded deals and random mixing.
- `crates/agent-avocado` is a deterministic, patient alpha-beta analyzer.
- `crates/agent-garlic` is Avocado's profile-guided, speed-focused successor.
- `crates/agent-iceberg` is a pressure-aware exact shortest-mate specialist.
- `crates/agent-cherry` is a policy/value MCTS analyzer learned from rules-only
  self-play.
- `crates/cherry-train` is Cherry's resumable native self-play trainer.
- `crates/agent-fajita` is a wide residual policy/value MCTS analyzer trained
  from independent fresh weights.
- `crates/agent-kiwi` uses Fajita's exact model and MCTS implementation with an
  independently published, continuously trained checkpoint.
- `crates/agent-arena` runs paired-seed round robins between the browser agents.
- `crates/fajita-train` is Fajita's high-quality, rules-only self-play trainer.
- `crates/kiwi-train` is Kiwi's ungated, rules-only continuous self-play trainer.
- `crates/snipe-wasm` is the browser bridge over Core and the six browser
  agents.
- `web` is the React game and analysis UI.

Cherry is the browser default, and the selected strategy controls both
computer play and informational analysis. Its browser checkpoint is frozen:
training is intentionally native and offline, while inference and MCTS remain
small enough to run in the existing Web Worker.

The repository-level `older_impls/` directory is retained as historical
material and is unrelated to the active Rust workspace.

## Run locally

Prerequisites:

- Rust with the `wasm32-unknown-unknown` target
- `wasm-pack`
- Node.js and npm

```sh
cd web
npm install
npm run dev
```

The development command rebuilds the clean WASM bridge before starting Vite.
This project is not deployed as part of ordinary development work.

## Verify

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

cd web
npm test
npm run build
```

## Run the browser-agent arena

From `rust_impl`, run the published Cherry, Garlic, Fajita, and Kiwi agents in
the default round robin:

```sh
cargo run --release -p agent-arena -- \
  --pairs 10 \
  --milliseconds 10000 \
  --save-results per-ply
```

Each of the six matchups runs concurrently. A matchup plays every seed twice,
with the agents swapping Alpha and Beta, so the command above plays 120 games.
The final table awards one point per win and half a point per draw.

To run only selected matchups, pass `--matchup AGENT-vs-AGENT` once per matchup.
Agent names are `avocado`, `cherry`, `fajita`, `garlic`, `iceberg`, and `kiwi`;
their order in the argument does not matter. For example, this runs only
Avocado versus Garlic:

```sh
cargo run --release -p agent-arena -- \
  --matchup avocado-vs-garlic \
  --pairs 10 \
  --milliseconds 10000 \
  --save-results per-ply
```

Omitting `--matchup` runs the default four-agent round robin. Avocado and Iceberg
remain available through an explicit `--matchup`. Repeat the flag to include
multiple selected matchups.

`--save-results` controls all tournament file output and has three modes:

- `per-ply` is the default. The arena creates the game file before the first
  ply, prefixes it with `// INCOMPLETE`, and atomically replaces it after every
  ply. Normal game completion removes the marker. If the process is interrupted,
  the marker and the last fully recorded ply remain.
- `per-game` writes one complete `.shgh` file after each game finishes.
- `off` does not create a tournament directory or write any files.

Saved results default to `agent-arena-results/`. When result saving is enabled,
the arena creates a new `tournament-*` directory containing `log.txt` and one
subdirectory for each selected pairing. Terminal output is duplicated to the
log. Use `--output-root PATH` to select another parent directory.
`--seed-start` changes the first paired seed, and `--max-plies` changes the draw
limit.

## Benchmark Iceberg

Iceberg ignores opening strength, material, and ordinary positional play. Its
tactical scout searches pressure-building attacks for a sound mate upper bound;
its exact search then disproves every shorter bound before reporting the
shortest mate as fully solved. The `game9` workload starts after ply 39—before
the annotated Alpha move—and prints both stages plus a concrete principal
variation:

```sh
cargo run --release -p agent-iceberg --example game9 -- 30000 --require-mate
```

## Benchmark Garlic

Garlic retains Avocado's search order and evaluation semantics while optimizing
the packed search path. Its strength benchmark requires both agents to reach the
same depth on seeded positions, asserts identical tick counts, evaluations, and
principal variations, then reports the wall-clock speedup:

```sh
cargo bench -p agent-garlic --bench strength
```

For sampling-profiler work, the longer fixed-search workload is:

```sh
cargo bench -p agent-garlic --bench search --profile profiling --no-run
```

## Train Cherry

Cherry starts from deterministic random weights and learns only from legal
self-play and the final winner. It does not consume game logs, human strategy
notes, other agent evaluations, material scores, or any
other hand-authored Snipe Hunt heuristic.

From `rust_impl`, start or resume the main run:

```sh
cargo run --release -p cherry-train -- train \
  --run-dir training/cherry-main \
  --hours 1000000 \
  --simulations 512 \
  --progress-reports on
```

`--progress-reports` defaults to `on`. Like Fajita, Cherry prints timestamped,
human-readable status reports every 500 games and an arena-specific promotion
or continuation report every 1,000 games. Use `--progress-reports off` for
quiet training; errors and graceful-shutdown confirmation are still printed.

The trainer makes the latest weights and run metadata durable after every
completed game, and flushes the replay window every 25 games. Press `Ctrl+C`
once to request a graceful shutdown: Cherry finishes the current self-play
batch or promotion arena, writes a full checkpoint including replay, and exits.
Press `Ctrl+C` a second time only when an immediate exit without saving is
preferable to waiting. Rerunning the same command resumes rather than starting
over. Self-play defaults to all but one available CPU core; pass `--workers N`
to cap both self-play and promotion arenas.

### Train Cherry on a 2021 14-inch M1 Pro MacBook Pro

From `rust_impl`, this command lets Cherry choose its default worker count,
compiles for the native M1 Pro CPU, and prevents idle system sleep:

```sh
caffeinate -i cargo run --release -p cherry-train -- train \
  --run-dir training/cherry-main \
  --hours 1000000 \
  --simulations 512
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. The million-hour training window is intentionally
effectively unbounded; press `Ctrl+C` once when it is time to move the run.
`caffeinate -i` ends after Cherry finishes its checkpoint and exits.

### Train Cherry on a 2017 13-inch Intel MacBook Pro

From `rust_impl`, this command lets Cherry choose its default worker count,
compiles for the native AVX2/FMA-capable CPU in a 2017 13-inch Intel MacBook
Pro, and prevents idle system sleep:

```sh
RUSTFLAGS="-C target-cpu=native" \
caffeinate -i cargo run --release -p cherry-train -- train \
  --run-dir training/cherry-main \
  --hours 1000000 \
  --simulations 512
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. The million-hour training window is intentionally
effectively unbounded; press `Ctrl+C` once when it is time to move the run.
`caffeinate -i` ends after Cherry finishes its checkpoint and exits.

### Move Cherry training between computers

The resumable run is ignored by Git, so stop Cherry and export it separately.
From `rust_impl`, create a timestamped, checksummed archive:

```sh
./scripts/export-cherry-training.sh
```

The archive defaults to the Git-ignored `training/exports/` directory. To put
it somewhere else, provide an explicit destination:

```sh
./scripts/export-cherry-training.sh ~/Desktop/cherry-main.tar.gz
```

After transferring the archive to the other computer, import it from that
computer's `rust_impl` directory:

```sh
./scripts/import-cherry-training.sh ~/Downloads/cherry-main.tar.gz
```

The import verifies every archived file, asks the local release trainer to
load the run, and only then installs it as `training/cherry-main`. If a local
run already exists, it is preserved under `training/backups/`. The workflow is
symmetric, so the same commands move the run back from the Intel MacBook.

Inspect or run a fresh paired arena:

```sh
cargo run --release -p cherry-train -- status --run-dir training/cherry-main
cargo run --release -p cherry-train -- evaluate \
  --run-dir training/cherry-main \
  --pairs 64
cargo run --release -p cherry-train -- audit \
  --run-dir training/cherry-main \
  --pairs 64
```

Automatic promotion uses paired deals with both agents playing each side. A
challenger becomes the staged champion only when its one-sided 95% lower
confidence bound exceeds 50%. Promotion checkpoints accumulate under
`league/`, and `arena.csv` records the strength trajectory. The always-current
`latest.bin` remains available for deliberately testing a very young model.
The audit pits the frozen network against a four-times-deeper search using the
same network, providing a reproducible search-based approximate exploitability
signal when there is no external expert benchmark.

Publishing is deliberately separate from training. It validates and copies the
latest staged checkpoint into the tracked browser model:

```sh
cargo run --release -p cherry-train -- publish --run-dir training/cherry-main
npm --prefix web run build:wasm
```

The publish command commits the browser checkpoint together with a minor web
version increment (resetting the patch to zero) in both `web/package.json` and
`web/package-lock.json`. The GUI reads that package version, so a successful
model publication cannot retain the preceding release's displayed version. By
default, publication first requires the entire Git worktree to be clean and
aborts before inspecting the training run when it is not. For an intentional
exception, pass `--allow-when-dirty` to the `publish` command.

## Train Fajita

Fajita has a 256-unit trunk and four residual layers. Its model, optimizer,
replay formats, initialization seed, and default run directory are independent.
A fresh Fajita run never consumes Cherry or Avocado weights or training data.

Fajita prioritizes self-play quality from game one. Its default search budget
matches Cherry's trainer: 512 simulations per action, raised for wide positions
to at least three times the legal branching factor and capped at 1,536.

```sh
cargo run --release -p fajita-train -- train \
  --run-dir training/fajita-main \
  --hours 1000000 \
  --progress-reports on

cargo run --release -p fajita-train -- status \
  --run-dir training/fajita-main
```

`--progress-reports` defaults to `on`. It prints timestamped, human-readable
status reports every 500 games, with arena-specific promotion, recovery, or
continuation reports every 1,000 games. Use `--progress-reports off` for quiet
training; errors are still written to stderr.

Training checkpoints are resumable. Fajita uses only internal paired-seed
arenas for champion promotion and regression recovery; external-agent
tournaments are intentionally outside this training sprint.
Fajita enters its lower-rate mature optimization phase after 150,000 updates,
which keeps later champion branches stable without altering their weights or
replay history.

Press `Ctrl+C` once to request a graceful shutdown. Fajita finishes the current
self-play batch or promotion arena, writes a full checkpoint including replay,
prints confirmation, and exits. Press `Ctrl+C` a second time only when an
immediate exit without saving is preferable to waiting.

### Train Fajita on a 2021 14-inch M1 Pro MacBook Pro

From `rust_impl`, this command lets Fajita choose its default worker count,
compiles for the native M1 Pro CPU, and prevents idle system sleep:

```sh
caffeinate -i cargo run --release -p fajita-train -- train \
  --run-dir training/fajita-main \
  --hours 1000000 \
  --progress-reports on
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. The million-hour training window is intentionally
effectively unbounded; stop the trainer manually when it is time to move the
run. `caffeinate -i` ends when the trainer exits.

### Train Fajita on a 2017 13-inch Intel MacBook Pro

From `rust_impl`, this command lets Fajita choose its default worker count,
compiles for the native AVX2/FMA-capable CPU in a 2017 13-inch Intel MacBook
Pro, and prevents idle system sleep:

```sh
RUSTFLAGS="-C target-cpu=native" \
caffeinate -i cargo run --release -p fajita-train -- train \
  --run-dir training/fajita-main \
  --hours 1000000 \
  --progress-reports on
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. The million-hour training window is intentionally
effectively unbounded; stop the trainer manually when it is time to move the
run. `caffeinate -i` ends when the trainer exits.

### Move Fajita training between computers

The resumable run is ignored by Git, so it must be transferred separately.
Always stop the trainer before exporting or importing. From `rust_impl`, create
a timestamped, checksummed archive:

```sh
./scripts/export-fajita-training.sh
```

By default the archive is written to the Git-ignored `training/exports/`
directory. An explicit destination may be supplied instead:

```sh
./scripts/export-fajita-training.sh ~/Desktop/fajita-main.tar.gz
```

Upload that archive to Google Drive. After downloading it on the other
computer, import it from that computer's `rust_impl` directory:

```sh
./scripts/import-fajita-training.sh ~/Downloads/fajita-main.tar.gz
```

The import checks every archived file, asks the local release trainer to load
the run, and only then installs it as `training/fajita-main`. If a run already
exists, it is retained under `training/backups/`. The workflow is symmetric:
to move training back, export on the Intel MacBook and import the resulting
archive on the original computer.

Publishing validates the run's fresh-seed purity marker and requires at least
one promoted champion. It always embeds `champion.bin`, never the potentially
unvalidated `latest.bin`. It also advances the web version as described above:

```sh
cargo run --release -p fajita-train -- publish \
  --run-dir training/fajita-main
npm --prefix web run build:wasm
```

After rebuilding WASM, Fajita is available in the browser's strategy selector.

## Train Kiwi

Kiwi is the controlled continuous-training counterpart to Fajita. It delegates
the network, encoding, optimizer, and MCTS implementation to `agent-fajita`, so
those algorithms cannot drift between the agents. A fresh Kiwi run deliberately
uses Fajita's same deterministic initialization and self-play RNG seed, along
with the same replay capacity, update schedule, learning rates, root noise, and
adaptive simulation budget.

The sole training-policy difference is the one under study: Kiwi always carries
the latest network forward. It runs no automatic champion match, promotion
decision, or regression rollback. Each new worker batch takes the latest
parameters available when that batch starts. This is an AlphaZero-style
continuous update loop, adapted to the trainer's single-machine batched worker
architecture rather than a claim to reproduce DeepMind's distributed TPU
scheduler.

```sh
cargo run --release -p kiwi-train -- train \
  --run-dir training/kiwi-main \
  --hours 1000000 \
  --progress-reports on

cargo run --release -p kiwi-train -- status \
  --run-dir training/kiwi-main
```

### Train Kiwi on a 2021 14-inch M1 Pro MacBook Pro

From `rust_impl`, this command lets Kiwi choose its default worker count,
compiles for the native M1 Pro CPU, and prevents idle system sleep:

```sh
caffeinate -i cargo run --release -p kiwi-train -- train \
  --run-dir training/kiwi-main \
  --hours 1000000 \
  --progress-reports on
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. The million-hour training window is intentionally
effectively unbounded; stop the trainer manually when it is time to move the
run. `caffeinate -i` ends when the trainer exits.

### Train Kiwi on a 2017 13-inch Intel MacBook Pro

From `rust_impl`, this command lets Kiwi choose its default worker count,
compiles for the native AVX2/FMA-capable CPU in a 2017 13-inch Intel MacBook
Pro, and prevents idle system sleep:

```sh
RUSTFLAGS="-C target-cpu=native" \
caffeinate -i cargo run --release -p kiwi-train -- train \
  --run-dir training/kiwi-main \
  --hours 1000000 \
  --progress-reports on
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. The million-hour training window is intentionally
effectively unbounded; stop the trainer manually when it is time to move the
run. `caffeinate -i` ends when the trainer exits.

Kiwi writes resumable `latest.bin`, optimizer, replay, and metadata checkpoints.
Every 1,000 games it also keeps an inert network snapshot under `snapshots/`.
Snapshots never affect self-play or training; they exist so regressions can be
measured after the fact without reintroducing a gate. The optional evaluator
compares the current network with any explicitly selected checkpoint and does
not modify the run:

```sh
cargo run --release -p kiwi-train -- evaluate \
  --run-dir training/kiwi-main \
  --against training/kiwi-main/snapshots/network-step-4000-game-1000.bin \
  --pairs 64
```

Removing the gate does not guarantee monotonic strength. MCTS uses the network's
policy priors and leaf values, so more search may damp some local errors but
cannot reliably repair a regressed network. The replay window and retained
snapshots provide stability and observability, respectively, without selecting
which network is allowed to generate self-play.

Press `Ctrl+C` once to finish the current worker batch and write a complete
checkpoint; press it a second time only for an immediate exit without saving.
Move a stopped run between computers with the Kiwi-specific checksummed scripts:

```sh
./scripts/export-kiwi-training.sh
./scripts/import-kiwi-training.sh ~/Downloads/kiwi-main-YYYYMMDDTHHMMSSZ.tar.gz
```

Publishing intentionally copies `latest.bin`; there is no champion artifact to
select. Until a Kiwi checkpoint is published, the browser's Kiwi option uses
the same deterministic fresh random initialization as a new Kiwi training run.
It does not load Fajita's trained checkpoint or any other training state. As
with Cherry and Fajita, publication advances the web version.

```sh
cargo run --release -p kiwi-train -- publish \
  --run-dir training/kiwi-main
npm --prefix web run build:wasm
```

## Browser behavior

- Rust/WASM is authoritative for dealing, legality, transitions, and analysis.
- Computer play and live analysis run in separate cancellable workers.
- Both use wall-clock budgets and report algorithm-neutral elapsed time and
  thinking ticks.
- Live analysis supports a committed first animal step without mutating history.
- Timelines, subply navigation, game mode, strategy, and time controls persist
  in versioned local storage.
- A WASM initialization failure is shown explicitly; there is no alternate
  JavaScript rules or search engine.
