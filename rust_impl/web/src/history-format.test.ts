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

describe("authoritative capture annotations", () => {
  it("does not infer a capture from frontend card-array changes", () => {
    expect(
      formatCompletedMove(move({ animals: [], snipe: null })),
    ).toBe("Snake 4");
  });

  it("uses Core's animal capture fact", () => {
    expect(
      formatCompletedMove(move({ animals: ["Squid"], snipe: null })),
    ).toBe("Snake 4x");
  });

  it("uses the captured snipe's original owner for the result suffix", () => {
    expect(
      formatCompletedMove(move({ animals: [], snipe: "Alpha" })),
    ).toBe("Snake 4-#0");
  });
});
