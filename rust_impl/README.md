# Snipe Hunt — clean Rust engine

The active implementation is intentionally small and dependency-directed:

- `crates/snipe-core` is the authoritative rules and public `Analyzer` contract.
- `crates/snipe-prng` owns reproducible seeded deals and random mixing.
- `crates/agent-avocado` is a deterministic, patient alpha-beta analyzer.
- `crates/agent-blueberry` is an aggressive, policy-guided Monte Carlo analyzer.
- `crates/agent-cherry` is a policy/value MCTS analyzer learned from rules-only
  self-play.
- `crates/cherry-train` is Cherry's resumable native self-play trainer.
- `crates/agent-eel` is a wider, deeper residual policy/value MCTS analyzer.
- `crates/eel-train` is Eel's resumable rules-only self-play trainer and
  reproducible external tournament runner.
- `crates/agent-fajita` is an independent fresh-weight clone of Eel's neural
  architecture.
- `crates/fajita-train` is Fajita's high-quality, rules-only self-play trainer.
- `crates/snipe-wasm` is the browser bridge over Core and the three agents.
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

## Train Eel

Eel starts from deterministic fresh random weights. Its trainer does not load
Cherry or Avocado weights, replay, evaluations, or training labels. Those older
agents are available only to the explicit tournament command.

```sh
cargo run --release -p eel-train -- nightly \
  --run-dir training/eel-main \
  --hours 8 \
  --simulations 128

cargo run --release -p eel-train -- status --run-dir training/eel-main
```

Eel has a 256-unit trunk with four residual layers, compared with Cherry's
128-unit trunk and one residual layer. The larger forward and backward passes
are an intentional capacity-for-walltime tradeoff.

After 200,000 optimizer steps the trainer halves its learning rate. If an
internal arena candidate scores below 45% against the validated champion, the
trainer restores that champion, resets Adam's moments, and retains the Eel-only
replay window. The same recovery can be requested explicitly with
`eel-train recover`.

Run the requested paired-seed handicap tournaments with durable game logs:
The runner fails fast after Eel's first loss or draw, because the required
perfect result is no longer attainable.

```sh
cargo run --release -p eel-train -- tournament \
  --run-dir training/eel-main \
  --opponent cherry \
  --checkpoint latest \
  --pairs 10 \
  --eel-ms 5000 \
  --older-ms 10000 \
  --log-dir ../eel_vs_cherry

cargo run --release -p eel-train -- tournament \
  --run-dir training/eel-main \
  --opponent avocado \
  --checkpoint latest \
  --pairs 10 \
  --eel-ms 5000 \
  --older-ms 10000 \
  --log-dir ../eel_vs_avocado
```

The run directory is ignored by Git; only an explicitly published checkpoint
is shipped. Self-play and search treat a repeated position or 256-action game
as a neutral training outcome so the workflow always terminates. This does not
add a draw rule to Snipe Hunt itself.

## Train Fajita

Fajita has Eel's 256-unit trunk and four residual layers, but its model,
optimizer, replay formats, initialization seed, and default run directory are
independent. A fresh Fajita run cannot load Eel checkpoints and never consumes
Eel, Cherry, or Avocado weights or training data.

Fajita prioritizes self-play quality from game one. Its default search budget
matches Cherry's trainer: 512 simulations per action, raised for wide positions
to at least three times the legal branching factor and capped at 1,536.

```sh
cargo run --release -p fajita-train -- nightly \
  --run-dir training/fajita-main \
  --hours 8

cargo run --release -p fajita-train -- status \
  --run-dir training/fajita-main
```

Training checkpoints are resumable. Fajita uses only internal paired-seed
arenas for champion promotion and regression recovery; external-agent
tournaments are intentionally outside this training sprint.
Fajita enters its lower-rate mature optimization phase after 150,000 updates,
which keeps later champion branches stable without altering their weights or
replay history.

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
