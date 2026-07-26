# Snipe Hunt — clean Rust engine

The active implementation is intentionally small and dependency-directed:

- `crates/snipe-core` is the authoritative rules and public `Analyzer` contract.
- `crates/agent-avocado` is a deterministic, patient alpha-beta analyzer.
- `crates/agent-blueberry` is an aggressive, policy-guided Monte Carlo analyzer.
- `crates/agent-cherry` is a policy/value MCTS analyzer learned from rules-only
  self-play.
- `crates/cherry-train` is Cherry's resumable native self-play trainer.
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

The run directory is ignored by Git; only an explicitly published checkpoint
is shipped. Self-play and search treat a repeated position or 256-action game
as a neutral training outcome so the workflow always terminates. This does not
add a draw rule to Snipe Hunt itself.

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
