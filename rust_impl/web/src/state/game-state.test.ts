import { describe, expect, it, vi } from "vitest";
import type { Position, RulesEngine, TurnMove } from "../engine/types";
import { gameReducer, newGame } from "./game-state";
import { restoreGame, saveGame } from "./persistence";

function position(positionKey: string, turn: Position["turn"] = "Alpha"): Position {
  return {
    schemaVersion: 1,
    positionKey,
    seed: 1,
    turn,
    turnNumber: turn === "Alpha" ? 1 : 2,
    winner: null,
    leadingAction: null,
    locations: {
      "alpha-reserve": [],
      "beta-reserve": [],
      "row-1": [],
      "row-2": [],
      "row-3": [],
      "row-4": [],
      "row-5": [],
      "row-6": [],
    },
  };
}

const initial = position("initial");
const after = position("after", "Beta");
const move: TurnMove = {
  id: "move",
  positionKey: initial.positionKey,
  player: "Alpha",
  label: "Rabbit 2",
  steps: [
    {
      pieceKey: "alpha:animal:3",
      animal: "Rabbit",
      owner: "Alpha",
      isSnipe: false,
      from: "row-1",
      to: "row-2",
      capture: { animals: [], snipe: null },
    },
  ],
  captures: { animals: [], snipe: null },
};

function engine(): RulesEngine {
  return {
    name: "test",
    createGame: vi.fn(() => initial),
    canonicalizePosition: (value) => value,
    legalMoves: vi.fn(() => [move]),
    previewFirstStep: vi.fn((value) => value),
    applyMove: vi.fn((base, candidate) => {
      if (
        base.positionKey !== initial.positionKey ||
        candidate.positionKey !== initial.positionKey
      ) {
        throw new Error("stale move");
      }
      return after;
    }),
  };
}

describe("game-state invariants", () => {
  it("uses Cherry for new games", () => {
    expect(newGame(initial).strategy).toBe("cherry");
  });

  it("refuses a commit calculated against a stale position", () => {
    const game = newGame(initial);
    const result = gameReducer(game, {
      type: "commit",
      basePositionKey: "another-position",
      position: after,
      move,
    });
    expect(result).toBe(game);
  });

  it("round-trips a replay-validated current-schema game", () => {
    const rules = engine();
    const committed = gameReducer(newGame(initial), {
      type: "commit",
      basePositionKey: initial.positionKey,
      position: after,
      move,
    });
    const restored = restoreGame(saveGame(committed), rules);
    expect(restored.timeline.map((entry) => entry.position.positionKey)).toEqual([
      "initial",
      "after",
    ]);
    expect(rules.applyMove).toHaveBeenCalledWith(initial, move);
  });

  it("persists Fajita as a selected strategy", () => {
    const rules = engine();
    const game = { ...newGame(initial), strategy: "fajita" as const };

    const restored = restoreGame(saveGame(game), rules);

    expect(restored.strategy).toBe("fajita");
  });

  it("persists Garlic as a selected strategy", () => {
    const rules = engine();
    const game = { ...newGame(initial), strategy: "garlic" as const };

    const restored = restoreGame(saveGame(game), rules);

    expect(restored.strategy).toBe("garlic");
  });

  it("discards old schemas instead of attempting a migration", () => {
    const rules = engine();
    const restored = restoreGame(
      JSON.stringify({ ...newGame(initial), schemaVersion: 0 }),
      rules,
    );
    expect(restored.timeline).toEqual([{ position: initial, move: null }]);
    expect(rules.createGame).toHaveBeenCalledOnce();
  });

  it("discards a terminal position whose contents fail canonical validation", () => {
    const rules = engine();
    vi.mocked(rules.legalMoves).mockImplementation((value) => {
      if (value.positionKey === "after") throw new Error("key mismatch");
      return [move];
    });
    const corrupted = gameReducer(newGame(initial), {
      type: "commit",
      basePositionKey: initial.positionKey,
      position: after,
      move,
    });
    const restored = restoreGame(saveGame(corrupted), rules);
    expect(restored.timeline).toEqual([{ position: initial, move: null }]);
  });
});
