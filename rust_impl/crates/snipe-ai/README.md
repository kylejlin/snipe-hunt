# snipe-ai

Native iterative-deepening negamax/PVS search for Snipe Hunt.

The searcher includes:

- hard deadline checks and a deterministic legal fallback;
- aspiration windows and principal-variation search;
- a generation-aware transposition table with mate-distance correction;
- tactical, TT, killer, and history move ordering;
- late-move reductions for quiet full turns;
- evaluation-guided selective width for the hundreds of paired-animal turns;
- capture/threat quiescence;
- repetition detection;
- search statistics and a legal principal variation;
- a tunable Snipe Hunt feature evaluation with explicit major-animal value
  and allegiance-aware capture pressure;
- a seat-swapping native match harness for engine and weight tournaments.

## `snipe-core` adapter contract

Implement `GamePosition` for a core-owned wrapper (Rust's orphan rules make a
wrapper preferable if both trait and state evolve independently). A search move
must represent one **complete turn** and always switch the side to move.

`terminal_score` and `evaluate` are from the current side's perspective. The
adapter should classify any full turn that captures animals, captures a snipe,
activates a triplet, or creates/answers an immediate snipe threat as tactical.
Its ordering score should prioritize, in order: immediate snipe capture,
preventing an immediate snipe capture, triplet captures by captured value,
forcing snipe threats, then quiet positional moves.

Feature extraction should populate `SnipeFeatures`; `SnipeWeights::default()`
provides the initial hand-tuned score. Native self-play should tune these
weights after rules parity is established.

## Strength smoke match

Run the timed searcher against a deterministic capture/threat greedy player:

```sh
cargo run --release -p snipe-ai --bin arena -- 10 50 300 greedy
```

Arguments are game count, milliseconds per search move, maximum turns, and
opponent (`greedy` or `random`). Seats swap every game and deals use
deterministic consecutive seeds.
