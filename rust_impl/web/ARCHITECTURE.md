# Web architecture

`web` is built on `snipe-core` through the Rust/WASM bridge.

## Authority boundaries

- `snipe-core` is the only rules authority.
- `snipe-prng` owns reproducible seeded deals; it does not define game rules.
- `snipe-wasm` converts complete Core states and actions to a narrow,
  value-semantic JSON contract.
- React renders engine snapshots and submits only engine-advertised turns. It
  never reconstructs legality, captures, ownership changes, or winning state.
- Search runs in disposable workers. Request IDs and canonical position keys
  prevent late worker responses from affecting a newer position.

## Identity

A card's game identity is `(allegiance, value, location)`. Identical copies in
one location are interchangeable in Core and therefore share a `pieceKey`.
There are no physical card IDs in state, moves, persistence, or history.

React separately gives each rendered occurrence a local key such as
`alpha:animal:3@row-2#1`. That key exists only long enough to highlight and
animate the exact tile the user clicked. The submitted action remains
value-semantic.

Positions use a collision-free canonical encoding of every Core card count,
the active player, and the optional leading action. Every turn carries the key
of the position for which it was generated. The WASM boundary rejects a stale
turn or any turn whose derived fields differ from Core's canonical turn.

## State and persistence

The reducer owns timelines, branches, navigation, draft subplies, and settings.
All commits are scoped to the current canonical position key.

Browser storage uses the private key `snipe-hunt.web.game` and schema 1.
Unsupported schemas and malformed games are discarded rather than migrated.
Restoration replays every move through Core and validates every stored
position, including the final one, before the state reaches React.

## Failure containment

WASM initialization failures have a dedicated startup screen. Unexpected
render failures are caught before further state can be committed and offer an
explicit way to discard only the web app's saved game. Worker cancellation terminates
the worker so synchronous Rust search cannot leak a late result.

## Local development

```sh
npm install
npm run dev
```

The displayed application version comes from `package.json`. Cherry, Fajita,
and Kiwi's trainer `publish` commands update the embedded model, `package.json`,
and `package-lock.json` as one release operation; each successful publication
advances the minor version and resets the patch to zero. Publication requires a
clean Git worktree unless its explicit `--allow-when-dirty` override is used.
Do not deploy this app as part of normal development.
