# Snipe Hunt — Rust engine

This is the Mission 7 implementation: a native Rust game/search engine, a
WebAssembly bridge, and a browser UI.

The implementation is split into:

- `crates/snipe-core`: authoritative game rules and state transitions
- `crates/snipe-ai`: timed search engine and match tooling
- `crates/snipe-wasm`: browser-facing WebAssembly API
- `web`: React UI and analysis worker

The older Java and TypeScript implementations remain in the repository as
rules and compatibility references.

## Run the web game

Prerequisites:

- Rust with the `wasm32-unknown-unknown` target
- `wasm-pack`
- Node.js and npm

```sh
cd web
npm install
npm run dev
```

`npm run dev` rebuilds the Rust engine into WebAssembly before starting Vite.
The production build is:

```sh
cd web
npm run build
```

## Verify

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

cd web
npm test
npm run build
```

## Measure playing strength

The native arena plays paired deals so both engines receive each side of the
same initial position:

```sh
cargo run --release -p snipe-ai --bin arena -- 20 1000 160 greedy 3
cargo run --release -p snipe-ai --bin arena -- 20 1000 160 random 3
```

The arguments are game count, milliseconds per search move, maximum game
length, opponent, and maximum search depth.

The original frozen depth-three baseline scored:

- 16 wins, 3 losses, and 1 draw against the tactical greedy baseline
- 20 wins and 0 losses against the seeded random baseline

These are deterministic regression matches across ten paired initial deals.
Production now lets iterative deepening use the full selected time budget
instead of stopping artificially at depth three. Against the frozen cap, the
uncapped engine scored 13–7 at 500 ms per move and 9–1 at one second per move.

Additional benchmark tools are available for adversarial validation:

```sh
# Compare two alpha-beta depth limits on mirrored deals.
cargo run --release -p snipe-ai --bin engine_arena -- 10 100 3 4 120

# Challenge alpha-beta with the independent deterministic MCTS player.
cargo run --release -p snipe-ai --bin mcts_arena -- 5 50 220

# Run training/holdout comparisons for evaluation-weight candidates.
cargo run --release -p snipe-ai --bin tune_weights -- 10 2 220 19
```

At very short 100 ms searches, frozen depth three beat depth four 12–8, which
is why time-budget comparisons are kept separate from fixed-depth experiments.
The MCTS challenger beat greedy 8–2 but lost 0–10 to the frozen depth-three
alpha-beta baseline at 50 ms per move. Evaluation-weight candidates failed
independent holdout, so production keeps the original weights.

The production search policy was selected with deterministic 5,000-node
matches across 30 mirrored pairs:

- direct-snipe-threat quiescence scored 48–12 against the frozen policy;
- exact-gated preservation of critical snipe escapes scored 35–25;
- the combined policy scored 49–11.

At a larger 20,000-node budget, close to the optimized WASM engine's
one-second workload, the combined policy also beat the frozen policy 13–7.

The defensive policy also fixes a reachable 129-move position where ordinary
48-move beam truncation discarded every move preventing immediate snipe
capture. See `tests/defensive_beam.rs` for the deterministic regression.

## Browser behavior

- The Rust/WASM engine is authoritative for dealing, legality, state
  transitions, and analysis.
- Search runs synchronously inside a dedicated worker. Cancelling analysis
  terminates and replaces the worker, keeping the UI responsive.
- The complete game timeline and analysis settings are versioned and stored in
  `localStorage`.
- Back and Forward navigate the timeline; moving from a prior position creates
  a new continuation.
