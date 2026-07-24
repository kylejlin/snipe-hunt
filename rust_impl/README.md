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

To build and publish the web app to GitHub Pages:

```sh
cd web
npm run deploy
```

The deployment script builds with the `/snipe-hunt/` base path and publishes
the generated app to the repository's `gh-pages` branch.

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

# Compare the bounded engine with a much deeper mirrored oracle.
cargo run --release -p snipe-ai --bin oracle_arena -- 2 20000 200000 80 0

# Mine replayable fast-versus-teacher decision labels.
cargo run --release -p snipe-ai --bin teacher_labels -- 6 1 12 4 20000 200000

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

Analysis requests also include the active game timeline. Search treats a
branch that re-enters a prior position as strongly unfavorable to the computer,
so it assumes a human opponent will exploit repetition while refusing to
settle for a cycle itself. `tests/repetition_cycle.rs` captures a real
production self-play loop and verifies that history-aware search avoids the
closing setup. The exact threat gate uses an early-exit atomic search verified
against exhaustive full-turn generation on thousands of reachable positions.

Quiescence also follows an otherwise quiet move when it closes an exact
repetition against the active path or supplied game history. This prevents the
closing move from disappearing just beyond an ordinary tactical frontier. The
focused regression changes the synthetic closing line from a neutral
stand-pat score to root-adverse repetition contempt. At a deterministic
20,000-node budget, the policy scored 21–19 against otherwise identical
production search across 20 mirrored pairs, so it was promoted without a
measurable strength regression.

Iterative deepening retains a fully completed aspiration-window result when
only its full-window verification search exhausts the move budget. Previously,
that deeper move was discarded and the engine fell back an entire completed
depth. The policy scored 23–17 at 5,000 nodes and 21–19 across two disjoint
20,000-node blocks, increasing average completed depth without increasing
search work. On a replayable teacher position, it reduced equal-depth oracle
regret from 1,131 to 689 evaluation points. Principal-variation extraction is
also anchored to the accepted root move, so a timed-out later search cannot
make browser analysis describe a different move.

In the optimized WebAssembly smoke test, this raises the reported completed
depth from 2 to 3 at one second and from 3 to 4 at five seconds, at essentially
the same 23,000-node and 95,000-node workloads.

Repetition identity canonicalizes the two strategically identical copies of
each animal type. A softer macro-history penalty also recognizes recurring
ownership and row-population structures without treating them as rule-level
draws. This converted a supported-snipe fortress from a 400-turn cap into an
Alpha win on turn 98. In a separate 20-pair deterministic tournament, the
macro-history policy scored 21–19 against the otherwise identical production
policy with equal search work per turn, showing no material strength
regression.

Capture-ordering and endgame-width experiments are intentionally not enabled.
Long-game audits showed that many apparent triplet captures merely recycled
the mover's own cards, but the full ownership-aware policy scored only 19–21
at 20,000 nodes and introduced a rational defensive repetition in one audited
seed. A 12-move dominant-material beam fixed that loop but lost 7–13 at the
same budget. Raising macro-history contempt fixed the audit set yet then lost
7–13 on the untouched holdout. Production therefore keeps the 48-move beam
and macro penalty 300 rather than promoting convergence gains that failed the
stronger playing-strength gate.

## Browser behavior

- The Rust/WASM engine is authoritative for dealing, legality, state
  transitions, and analysis.
- The timeline is supplied to analysis so repeated-position cycles are avoided.
- Computer play uses a timed, one-shot worker. Impartial analysis uses a
  separate depth-limited worker and publishes every completed iterative-
  deepening result, so both searches can run independently.
- Analysis can constrain its root to a committed first animal step and then
  suggest the best legal second subply without changing the game.
- The complete game timeline, game mode, agent time, and analysis settings are
  versioned and stored in `localStorage`.
- Back and Forward navigate the timeline; moving from a prior position creates
  a new continuation.
