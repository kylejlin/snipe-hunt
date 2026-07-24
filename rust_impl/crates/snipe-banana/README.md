# Banana

Banana is the production Snipe Hunt player. It is independent of the generic
Almond (`snipe-ai`) search and specializes its hot path for `snipe_core::State`.

The design priorities are:

- exact two-substep snipe-capture detection at the search horizon;
- exact preservation of replies to an immediate snipe-capture threat;
- apply and evaluate each generated child once;
- a narrow, strategically ordered beam instead of exhaustive exploration of
  hundreds of similar two-animal permutations;
- direct evaluation of material (with extra major value), rank pressure and
  control, snipe sanctuaries, trench support, and retreater breakthroughs;
- iterative deepening, PVS, a direct-mapped transposition table, repetition
  contempt with interchangeable animal copies canonicalized, and hard time
  limits.

Run an equal-time paired arena:

```sh
cargo run --release -p snipe-banana --bin banana_arena -- 10 100 160 0 48
```

The sixth optional argument gives Almond a different budget and the seventh
optionally caps Banana's depth. Major iterations first run a 5-second equal-
time presmoke:

```sh
cargo run --release -p snipe-banana --bin banana_arena -- 1 5000 100 0 48 5000
```

Only iterations that consistently win that gate advance to the required
asymmetric strength smoke test:

```sh
cargo run --release -p snipe-banana --bin banana_arena -- 1 5000 100 0 48 30000
```

This plays the same deal twice with seats swapped. It is intentionally slow:
Banana receives five seconds per turn and Almond receives thirty.
