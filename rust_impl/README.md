# Snipe Hunt — clean Rust engine

The active implementation is intentionally small and dependency-directed:

- `crates/snipe-core` is the authoritative rules and public `Analyzer` contract.
- `crates/snipe-prng` owns reproducible seeded deals and random mixing.
- `crates/agent-avocado` is a deterministic, patient alpha-beta analyzer.
- `crates/agent-garlic` is Avocado's profile-guided, speed-focused successor.
- `crates/agent-cherry` is a policy/value MCTS analyzer learned from rules-only
  self-play.
- `crates/cherry-train` is Cherry's resumable native self-play trainer.
- `crates/agent-fajita` is a wide residual policy/value MCTS analyzer trained
  from independent fresh weights.
- `crates/agent-arena` runs paired-seed round robins between the browser agents.
- `crates/fajita-train` is Fajita's high-quality, rules-only self-play trainer.
- `crates/snipe-wasm` is the browser bridge over Core and the three browser
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

From `rust_impl`, run the published Avocado, Cherry, Fajita, and Garlic agents in a
round robin:

```sh
cargo run --release -p agent-arena -- \
  --pairs 10 \
  --milliseconds 10000 \
  --save-games per-ply
```

Each of the six matchups runs concurrently. A matchup plays every seed twice,
with the agents swapping Alpha and Beta, so the command above plays 120 games.
The final table awards one point per win and half a point per draw.

`--save-games` has three modes:

- `per-ply` is the default. The arena creates the game file before the first
  ply, prefixes it with `// INCOMPLETE`, and atomically replaces it after every
  ply. Normal game completion removes the marker. If the process is interrupted,
  the marker and the last fully recorded ply remain.
- `per-game` writes one complete `.shgh` file after each game finishes.
- `off` does not create history directories or files.

Saved histories default to `agent-arena-results/`. Every invocation creates a
new `tournament-*` directory, with `avocado-vs-cherry`,
`avocado-vs-fajita`, `avocado-vs-garlic`, `cherry-vs-fajita`,
`cherry-vs-garlic`, and `fajita-vs-garlic` subdirectories. Use
`--output-root PATH` to select another parent directory. `--seed-start` changes
the first paired seed, and `--max-plies` changes the draw limit.

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
  --simulations 512
```

The trainer makes the latest weights and run metadata durable after every
completed game, and flushes the replay window every 25 games. Press `Ctrl+C`
once to request a graceful shutdown: Cherry finishes the current self-play
batch or promotion arena, writes a full checkpoint including replay, and exits.
Press `Ctrl+C` a second time only when an immediate exit without saving is
preferable to waiting. Rerunning the same command resumes rather than starting
over. Self-play defaults to all but one available CPU core; pass `--workers N`
to cap both self-play and promotion arenas.

### Train Cherry on a 13-inch Intel MacBook Pro

From `rust_impl`, this thermal-conscious command caps Cherry at three worker
threads, leaves CPU headroom on a dual-core 2017 13-inch Intel MacBook Pro,
compiles for its native AVX2/FMA-capable CPU, and prevents idle system sleep:

```sh
RUSTFLAGS="-C target-cpu=native" \
caffeinate -i cargo run --release -p cherry-train -- train \
  --run-dir training/cherry-main \
  --hours 1000000 \
  --simulations 512 \
  --workers 3
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

### Train on a 13-inch Intel MacBook Pro

From `rust_impl`, this thermal-conscious command caps both self-play and
promotion arenas at three worker threads, leaving CPU headroom on a dual-core
2017 13-inch Intel MacBook Pro, compiles for its native AVX2/FMA-capable CPU,
and prevents idle system sleep:

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
unvalidated `latest.bin`:

```sh
cargo run --release -p fajita-train -- publish \
  --run-dir training/fajita-main
npm --prefix web run build:wasm
```

After rebuilding WASM, Fajita is available in the browser's strategy selector.

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
