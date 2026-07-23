import { describe, expect, it } from "vitest";
import {
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
} from "./engine/fallback-core";
import type { TurnMove } from "./engine/types";
import {
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
      "2b. Beta 6R",
      "2a. Alpha 1R",
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
    ).toBe(`${betaRetreater!.animal} 6R`);
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
    ).toBe(`${betaOther!.animal} 4, ${betaRetreater!.animal} 6R`);
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
    ["legacy backward suffix", (history: string) => history.replace(/(\d)R/, "$1B")],
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

  it("rejects embedded blank lines without changing parser state", () => {
    const history = serializeHistory([{ position: createFallbackGame(7_071), move: null }]);
    expect(() => parseHistory(history.replace("\n0a.", "\n\n0a."), fallbackEngine)).toThrow(
      "blank lines",
    );
  });
});
