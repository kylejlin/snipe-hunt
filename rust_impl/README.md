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

The production depth-three configuration scored:

- 16 wins, 3 losses, and 1 draw against the tactical greedy baseline
- 20 wins and 0 losses against the seeded random baseline

These are deterministic regression matches across ten paired initial deals.

Additional benchmark tools are available for adversarial validation:

```sh
# Compare two alpha-beta depth limits on mirrored deals.
cargo run --release -p snipe-ai --bin engine_arena -- 10 100 3 4 120

# Challenge alpha-beta with the independent deterministic MCTS player.
cargo run --release -p snipe-ai --bin mcts_arena -- 5 50 220

# Run training/holdout comparisons for evaluation-weight candidates.
cargo run --release -p snipe-ai --bin tune_weights -- 10 2 220 19
```

In the current regression set, depth three beat depth four 12–8 at 100 ms per
move. The MCTS challenger beat greedy 8–2 but lost 0–10 to depth-three
alpha-beta at 50 ms per move. A weight candidate that looked stronger at depth
one regressed to 5–11–4 on the independent depth-two holdout, so production
keeps the original evaluation weights.

## Browser behavior

- The Rust/WASM engine is authoritative for dealing, legality, state
  transitions, and analysis.
- Search runs synchronously inside a dedicated worker. Cancelling analysis
  terminates and replaces the worker, keeping the UI responsive.
- The complete game timeline and analysis settings are versioned and stored in
  `localStorage`.
- Back and Forward navigate the timeline; moving from a prior position creates
  a new continuation.
