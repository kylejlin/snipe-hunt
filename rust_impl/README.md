# Snipe Hunt — clean Rust engine

The active implementation is intentionally small and dependency-directed:

- `crates/snipe-core` is the authoritative rules and public `Analyzer` contract.
- `crates/snipe-prng` owns reproducible seeded deals and random mixing.
- `crates/agent-avocado` is a deterministic, patient alpha-beta analyzer.
- `crates/agent-blueberry` is an aggressive, policy-guided Monte Carlo analyzer.
- `crates/agent-cherry` is a policy/value MCTS analyzer learned from rules-only
  self-play.
- `crates/cherry-train` is Cherry's resumable native self-play trainer.
- `crates/agent-fajita` is a wide residual policy/value MCTS analyzer trained
  from independent fresh weights.
- `crates/fajita-train` is Fajita's high-quality, rules-only self-play trainer.
- `crates/snipe-wasm` is the browser bridge over Core and the four browser
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

## Train Cherry

Cherry starts from deterministic random weights and learns only from legal
self-play and the final winner. It does not consume game logs, human strategy
notes, Avocado evaluations, Blueberry evaluations, material scores, or any
other hand-authored Snipe Hunt heuristic.

From `rust_impl`, start or resume the main run:

```sh
cargo run --release -p cherry-train -- nightly \
  --run-dir training/cherry-main \
  --hours 8 \
  --simulations 24
```

The trainer makes the latest weights and run metadata durable after every
completed game, and flushes the replay window every ten games. Stop it with
Ctrl-C at any time; rerunning the same command resumes rather than starting
over. No completed weight update is lost, while up to nine games may be absent
from the restored replay window. Self-play defaults to all but one available
CPU core; pass `--workers N` to override it.

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
cargo run --release -p fajita-train -- nightly \
  --run-dir training/fajita-main \
  --hours 8 \
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

### Train on a 13-inch Intel MacBook Pro

From `rust_impl`, this thermal-conscious command caps both self-play and
promotion arenas at three worker threads, leaving CPU headroom on a quad-core
13-inch Intel MacBook Pro, and prevents idle system sleep:

```sh
caffeinate -i cargo run --release -p fajita-train -- nightly \
  --run-dir training/fajita-main \
  --hours 8 \
  --workers 3 \
  --progress-reports on
```

Keep the MacBook plugged in with its lid open and unobstructed ventilation.
The display may sleep. `caffeinate -i` ends when the trainer exits, and the
trainer writes a completion report after its training window finishes.

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
