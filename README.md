# Snipe Hunt

Snipe Hunt is a two-player card game.
I've prototyped a JavaFX implementation, as well as a TypeScript implementation.
There may be bugs.

## Rules

The JavaFX implementation contains a [user guide](./java_impl/Snipe%20Hunt%20JavaFX%20Guide.pdf), which includes a rulebook.

## Roadmap

Mission 7 is implemented in [`rust_impl`](./rust_impl): an authoritative Rust
rules engine, a tuned alpha-beta computer player compiled to WebAssembly, and a
modern browser interface. The original Java and TypeScript versions remain as
historical references.

See [`rust_impl/README.md`](./rust_impl/README.md) for development, testing, and
strength-benchmark commands.
