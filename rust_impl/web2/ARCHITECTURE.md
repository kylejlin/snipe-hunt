# Web2 architecture

`web2` is a separate application built directly on `snipe-core`. It does not
load, migrate, or share browser state with `web`.

## Authority boundaries

- `snipe-core` is the only rules authority.
- `snipe-web2-wasm` converts complete Core states and actions to a narrow,
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

Browser storage uses the private key `snipe-hunt.web2.game` and schema 1.
Unsupported schemas and malformed games are discarded rather than migrated.
Restoration replays every move through Core and validates every stored
position, including the final one, before the state reaches React.

## Failure containment

WASM initialization failures have a dedicated startup screen. Unexpected
render failures are caught before further state can be committed and offer an
explicit way to discard only web2's saved game. Worker cancellation terminates
the worker so synchronous Rust search cannot leak a late result.

## Local development

```sh
npm install
npm run dev
```

The displayed application version comes from `package.json`. Do not deploy
this app as part of normal development.
