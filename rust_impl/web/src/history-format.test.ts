import { describe, expect, it } from "vitest";
import {
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
} from "./engine/fallback-core";
import type { TurnMove } from "./engine/types";
import {
  formatDisplayPlyPrefix,
  formatMove,
  parseHistory,
  serializeHistory,
  type TimelineEntry,
} from "./history-format";

const fallbackEngine = {
  legalMoves: fallbackLegalMoves,
  applyMove: applyFallbackMove,
};

function move(
  player: "Alpha" | "Beta",
  steps: TurnMove["steps"],
): TurnMove {
  return { id: "test", player, label: "", steps, captures: [] };
}

describe("compact history notation", () => {
  it("imports and reproduces the documented sample", () => {
    const sample = [
      "0b. =Monkey; Squid Frog Beta; Ox Tiger Rabbit Dragon Horse Elephant Rat Ox Snake Dog Boar Frog; Dragon",
      "0a. =Fish; Rabbit Fish Alpha; Rat Snake Ram Monkey Rooster Dog Boar Tiger Ram Rooster Elephant Squid; Horse",
      "1b. Beta 5",
      "1a. Alpha 2",
      "2b. Beta 6*",
      "2a. Alpha 1*",
      "",
    ].join("\n");

    const timeline = parseHistory(sample, fallbackEngine);
    expect(timeline).toHaveLength(5);
    expect(serializeHistory(timeline)).toBe(sample);
  });

  it("formats advances, retreats, drops, snipes, and two-step plies", () => {
    const position = createFallbackGame(7_071);
    const betaRetreater = position.locations["row-5"].find((card) => card.canRetreat);
    const betaReserve = position.locations["beta-reserve"][0];
    const betaSnipe = position.locations["row-6"].find((card) => card.isSnipe);
    const betaOther = position.locations["row-5"].find(
      (card) => card.id !== betaRetreater?.id,
    );
    expect(betaRetreater && betaReserve && betaSnipe && betaOther).toBeTruthy();

    expect(
      formatMove(
        position,
        move("Beta", [{ cardId: betaRetreater!.id, from: "row-5", to: "row-4" }]),
      ),
    ).toBe(`${betaRetreater!.animal} 4`);
    expect(
      formatMove(
        position,
        move("Beta", [{ cardId: betaRetreater!.id, from: "row-5", to: "row-6" }]),
      ),
    ).toBe(`${betaRetreater!.animal} 6*`);
    expect(
      formatMove(
        position,
        move("Beta", [
          { cardId: betaReserve!.id, from: "beta-reserve", to: "row-4" },
        ]),
      ),
    ).toBe(`${betaReserve!.animal} 4!`);
    expect(
      formatMove(
        position,
        move("Beta", [{ cardId: betaSnipe!.id, from: "row-6", to: "row-5" }]),
      ),
    ).toBe("Beta 5");
    expect(
      formatMove(
        position,
        move("Beta", [
          { cardId: betaOther!.id, from: "row-5", to: "row-4" },
          { cardId: betaRetreater!.id, from: "row-5", to: "row-6" },
        ]),
      ),
    ).toBe(`${betaOther!.animal} 4, ${betaRetreater!.animal} 6*`);
  });

  it("round-trips a complete active timeline", () => {
    let position = createFallbackGame(7_071);
    const timeline: TimelineEntry[] = [{ position, move: null }];
    for (let index = 0; index < 4; index += 1) {
      const legalMoves = fallbackLegalMoves(position);
      const selected =
        legalMoves.find((candidate) => candidate.steps[0].cardId.endsWith("-snipe")) ??
        legalMoves[0];
      position = applyFallbackMove(position, selected);
      timeline.push({ position, move: selected });
    }

    const exported = serializeHistory(timeline);
    const imported = parseHistory(exported, fallbackEngine);

    expect(serializeHistory(imported)).toBe(exported);
    expect(imported).toHaveLength(timeline.length);
    expect(imported.at(-1)?.position.turn).toBe(position.turn);
  });

  it.each([
    ["uppercase player prefix", (history: string) => history.replace("1b.", "1B.")],
    ["Greek player prefix", (history: string) => history.replace("1b.", "1β.")],
    ["legacy retreat suffix", (history: string) => history.replace(/(\d)\*/, "$1R")],
  ])("rejects %s", (_name, mutate) => {
    let position = createFallbackGame(7_071);
    const first = fallbackLegalMoves(position).find(
      (candidate) =>
        candidate.steps[0].from === "row-5" && candidate.steps[0].to === "row-6",
    );
    expect(first).toBeTruthy();
    position = applyFallbackMove(position, first!);
    const history = serializeHistory([
      { position: createFallbackGame(7_071), move: null },
      { position, move: first! },
    ]);

    expect(() => parseHistory(mutate(history), fallbackEngine)).toThrow();
  });

  it("uses Greek player letters only in display prefixes", () => {
    expect(formatDisplayPlyPrefix(0, "Beta")).toBe("0β.");
    expect(formatDisplayPlyPrefix(3, "Alpha")).toBe("3α.");
    expect(formatDisplayPlyPrefix(12, "Alpha", true)).toBe("12.5α.");
    expect(serializeHistory([{ position: createFallbackGame(7_071), move: null }])).toMatch(
      /^0b\.[^\n]*\n0a\./,
    );
  });

  it("ignores blank lines and single-line comments without changing parser state", () => {
    let position = createFallbackGame(7_071);
    const first = fallbackLegalMoves(position)[0];
    position = applyFallbackMove(position, first);
    const history = serializeHistory([
      { position: createFallbackGame(7_071), move: null },
      { position, move: first },
    ]);
    const annotated = `// Game notes

${history
  .replace("\n0a.", "\n\n// Alpha layout follows\n0a.")
  .replace("\n1b.", "\n\n// First move\n1b.")}

// End of game
`;

    expect(serializeHistory(parseHistory(annotated, fallbackEngine))).toBe(history);
  });

  it("reports physical line numbers after ignored lines", () => {
    const history = serializeHistory([{ position: createFallbackGame(7_071), move: null }]);
    const invalid = `// Header

${history}1b. Not a move
`;

    expect(() => parseHistory(invalid, fallbackEngine)).toThrow("Line 5:");
  });
});
