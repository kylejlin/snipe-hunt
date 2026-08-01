import { describe, expect, it } from "vitest";
import type { Position, TurnMove } from "./engine/types";
import { formatCompletedMove } from "./history-format";

const position: Position = {
  schemaVersion: 1,
  positionKey: "position",
  seed: 1,
  turn: "Beta",
  turnNumber: 1,
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

function move(capture: TurnMove["captures"]): TurnMove {
  return {
    id: "snake-4",
    positionKey: position.positionKey,
    player: "Beta",
    label: "Snake 4",
    steps: [
      {
        pieceKey: "beta:animal:5",
        animal: "Snake",
        owner: "Beta",
        isSnipe: false,
        from: "row-5",
        to: "row-4",
        capture,
      },
    ],
    captures: capture,
  };
}

describe("authoritative move annotations", () => {
  it("does not infer a capture from frontend card-array changes", () => {
    expect(
      formatCompletedMove(move({ animals: [], snipe: null }), null),
    ).toBe("Snake 4");
  });

  it("uses Core's animal capture fact", () => {
    expect(
      formatCompletedMove(move({ animals: ["Squid"], snipe: null }), null),
    ).toBe("Snake 4x");
  });

  it("uses the resulting winner for a snipe-capture result suffix", () => {
    expect(
      formatCompletedMove(move({ animals: [], snipe: "Alpha" }), "Beta"),
    ).toBe("Snake 4-#0");
  });

  it("uses the resulting winner for an immobilization result suffix", () => {
    expect(
      formatCompletedMove(move({ animals: [], snipe: null }), "Alpha"),
    ).toBe("Snake 4+#0");
  });
});
