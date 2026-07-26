import { describe, expect, it } from "vitest";
import type { Card, Position, TurnMove } from "./engine/types";
import { formatCompletedMove } from "./history-format";

const card = (
  id: string,
  animal: string,
  owner: Card["owner"],
  isSnipe = false,
): Card => ({
  id,
  animal,
  owner,
  isSnipe,
  canRetreat: animal === "Snake",
});

const position = (
  betaReserve: Card[],
  row4: Card[],
  row5: Card[],
): Position => ({
  schemaVersion: 1,
  seed: 1,
  turn: "Beta",
  turnNumber: 1,
  winner: null,
  locations: {
    "alpha-reserve": [],
    "beta-reserve": betaReserve,
    "row-1": [],
    "row-2": [],
    "row-3": [],
    "row-4": row4,
    "row-5": row5,
    "row-6": [],
  },
});

const snakeMove: TurnMove = {
  id: "snake-4",
  player: "Beta",
  label: "Snake 4",
  steps: [{ cardId: "snake", from: "row-5", to: "row-4" }],
  captures: [],
};

describe("capture annotations", () => {
  it("does not invent a capture when stable card IDs are reassigned", () => {
    const before = position(
      [card("animal-9", "Rooster", "Beta")],
      [card("squid", "Squid", "Beta")],
      [card("snake", "Snake", "Beta")],
    );
    const after = position(
      [card("animal-25", "Rooster", "Beta")],
      [
        card("squid", "Squid", "Beta"),
        card("snake", "Snake", "Beta"),
      ],
      [],
    );

    expect(formatCompletedMove(before, snakeMove, after)).toBe("Snake 4");
  });

  it("annotates an actual increase in the moving player's reserve", () => {
    const before = position(
      [card("rooster-before", "Rooster", "Beta")],
      [card("squid", "Squid", "Alpha")],
      [card("snake", "Snake", "Beta")],
    );
    const after = position(
      [
        card("rooster-after", "Rooster", "Beta"),
        card("captured-squid", "Squid", "Beta"),
      ],
      [card("snake", "Snake", "Beta")],
      [],
    );

    expect(formatCompletedMove(before, snakeMove, after)).toBe("Snake 4x");
  });
});
