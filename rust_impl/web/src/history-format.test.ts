import { describe, expect, it } from "vitest";
import type {
  MoveStep,
  Position,
  RulesEngine,
  TurnMove,
} from "./engine/types";
import {
  formatCompletedMove,
  formatDisplayPlyPrefix,
  parseHistory,
  serializeHistory,
} from "./history-format";

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

const importEngine = {
  canonicalizePosition: (candidate: Position) => ({
    ...candidate,
    positionKey: "canonical",
  }),
  legalMoves: () => [],
  previewFirstStep: (candidate: Position, _step: MoveStep) => candidate,
  applyMove: (candidate: Position, _move: TurnMove) => candidate,
} satisfies Pick<
  RulesEngine,
  "canonicalizePosition" | "legalMoves" | "previewFirstStep" | "applyMove"
>;

const balancedHistory = [
  "0b. =Rooster; Ram Frog Beta; Rat Ox Tiger Tiger Rabbit Dragon Snake Monkey Dog Elephant Squid Frog; Squid",
  "0a. =Dragon; Snake Boar Alpha; Rat Ox Rabbit Horse Ram Monkey Rooster Dog Boar Fish Fish Elephant; Horse",
].join("\n");
const imbalancedHistory = balancedHistory
  .replace("0b. =Rooster", "0b. =Dragon")
  .replace("0a. =Dragon", "0a. =Rooster");
const annotatedImbalancedHistory = imbalancedHistory
  .replace("0b. =Dragon", "0b. (+1 major) =Dragon")
  .replace("0a. =Rooster", "0a. (-1 major) =Rooster");

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
  it("uses Greek player markers without half-ply notation in the UI", () => {
    expect(formatDisplayPlyPrefix(3, "Alpha")).toBe("3α.");
    expect(formatDisplayPlyPrefix(4, "Beta")).toBe("4β.");
  });

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

describe("initial Major Animal balance", () => {
  it("rejects an imbalance without annotations", () => {
    expect(() => parseHistory(imbalancedHistory, importEngine)).toThrow(
      /Beta has 5 Major Animals; expected the annotation "\(\+1 major\)"/,
    );
  });

  it("accepts exact annotations and preserves them when exporting", () => {
    const timeline = parseHistory(annotatedImbalancedHistory, importEngine);

    expect(timeline).toHaveLength(1);
    expect(serializeHistory(timeline)).toBe(`${annotatedImbalancedHistory}\n`);
  });

  it("rejects a false-positive annotation on a balanced layout", () => {
    const falsePositive = balancedHistory.replace(
      "0b. =Rooster",
      "0b. (+1 major) =Rooster",
    );

    expect(() => parseHistory(falsePositive, importEngine)).toThrow(
      /Beta has exactly 4 Major Animals, so an imbalance annotation is not allowed/,
    );
  });

  it("rejects annotations whose amounts do not match the layout", () => {
    const wrongAmount = annotatedImbalancedHistory.replace(
      "0b. (+1 major)",
      "0b. (+2 major)",
    );

    expect(() => parseHistory(wrongAmount, importEngine)).toThrow(
      /annotated Major Animal imbalance \+2 does not match Beta's actual imbalance \+1/,
    );
  });

  it("requires an annotation for each imbalanced player", () => {
    const missingAlphaAnnotation = annotatedImbalancedHistory.replace(
      "0a. (-1 major)",
      "0a.",
    );

    expect(() => parseHistory(missingAlphaAnnotation, importEngine)).toThrow(
      /Alpha has 3 Major Animals; expected the annotation "\(-1 major\)"/,
    );
  });

  it("does not bypass creature conservation for an annotated imbalance", () => {
    const missingRooster = annotatedImbalancedHistory.replace(
      "0a. (-1 major) =Rooster",
      "0a. (-1 major) =Ox",
    );

    expect(() => parseHistory(missingRooster, importEngine)).toThrow(
      /more than two Ox cards/,
    );
  });
});
