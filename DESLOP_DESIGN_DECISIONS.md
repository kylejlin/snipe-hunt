# Deslopped Core Design Decisions

> **Notice:** This handoff summary was AI-generated from the project's current header and the design discussion with the project owner.

## Purpose and scope

- `deslopped-core` is a new authoritative reference implementation built from the ground up.
- It prioritizes a clean public interface, developer ergonomics, and correctness over time or space efficiency.
- The existing core and AI implementations are references for current behavior, not architectural constraints on the replacement.
- The WASM glue and frontend may be changed as needed to fit the new core. Preserving their current internal DTOs and adapter interfaces is not required.
- The finished replacement should still support the current webapp's user-facing capabilities.

## Rules and state model

- The public rules API is action-based. It does not need a separate whole-ply type.
- `State::apply` applies one atomic `Action`.
- Animal turns normally consist of two sequential `AnimalStep` actions.
- A first animal step may win immediately, in which case there is no second action. Consumers should detect this from the resulting state's `winner()`.
- Snipe steps and drops are standalone plies. `Action::is_standalone_ply` identifies these cases.
- `leading_action` represents a committed first animal action and lets the same state API model partial plies.
- The core stores the two physical copies of each animal as multiplicities rather than distinct card identities.
- Stable visual card IDs, animation identities, and similar presentation concerns belong in the WASM/frontend layer. They do not need to shape the authoritative rules representation.
- Alpha and Beta reserves are combined in the `State` representation. Card allegiance is sufficient to interpret reserve membership under the game rules.

## State construction and validation

- Random or seeded dealing does not need to be part of the core header. The caller or glue layer may generate a deal and pass it through `InitialStateBuilder`.
- `State` currently exposes its fields publicly for ergonomic construction and inspection.
- Direct construction of arbitrary midgame states is consequently a trusted operation unless validation is added later. A consumer that needs a strict persistence boundary may instead store an initial state and replay validated actions.

## Analyzer redesign

- The current AI implementations will be discarded rather than migrated.
- The replacement analyzer will be written from scratch and based on alpha-beta pruning.
- Efficient analyzers should use their own internal state representation and convert to or from `deslopped-core::State` only at public interface boundaries.
- The analyzer does not need game-history input merely to preserve the current AIs' repetition or convergence policies. Those policies are not requirements for the new implementation.
- Runtime analyzer selection does not require an object-safe trait. An enum will provide dispatch.
- The UI will display thinking ticks rather than completed search depth, so the analyzer API does not need to report depth.
- `think` uses `on_tick_complete` for stopping policies such as a tick budget, elapsed wall-clock time, or both.
- The optimal line of play is emitted as atomic `Action`s. It does not require a separate ply representation.

## Errors and implementation freedom

- `IllegalActionError` is intentionally only a placeholder in the header.
- The implementation agent should add and refine error variants as required by rule validation and useful diagnostics.
- The exact error taxonomy is not a frozen design decision.
- Nontrivial implementations belong in `private_impl`; `lib.rs` serves as the concise public-interface “header.”
