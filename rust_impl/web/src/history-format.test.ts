import { describe, expect, it } from "vitest";
import {
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
  previewFallbackFirstStep,
} from "./engine/fallback-core";
import type { TurnMove } from "./engine/types";
import {
  formatCompletedMove,
  formatDisplayPlyPrefix,
  formatMove,
  parseHistory,
  serializeHistory,
  snipeCaptureSuffix,
  type TimelineEntry,
} from "./history-format";

const fallbackEngine = {
  legalMoves: fallbackLegalMoves,
  previewFirstStep: previewFallbackFirstStep,
  applyMove: applyFallbackMove,
};

function move(
  player: "Alpha" | "Beta",
  steps: TurnMove["steps"],
): TurnMove {
  return { id: "test", player, label: "", steps, captures: [] };
}

function betaCaptureTimeline(): TimelineEntry[] {
  let position = createFallbackGame(7_071);
  const timeline: TimelineEntry[] = [{ position, move: null }];
  const attacker = position.locations["row-5"].find((card) => !card.isSnipe);
  expect(attacker).toBeTruthy();

  const play = (predicate: (candidate: TurnMove) => boolean) => {
    const selected = fallbackLegalMoves(position).find(predicate);
    expect(selected).toBeTruthy();
    position = applyFallbackMove(position, selected!);
    timeline.push({ position, move: selected! });
  };

  play((candidate) => candidate.steps[0].cardId === attacker!.id && candidate.steps[0].to === "row-4");
  play((candidate) => candidate.steps[0].cardId === "alpha-snipe" && candidate.steps[0].to === "row-2");
  play((candidate) => candidate.steps[0].cardId === attacker!.id && candidate.steps[0].to === "row-3");
  play((candidate) => !candidate.steps[0].cardId.endsWith("-snipe"));
  play((candidate) => candidate.steps[0].cardId === attacker!.id && candidate.steps[0].to === "row-2");

  expect(position.winner).toBe("Beta");
  return timeline;
}

describe("compact history notation", () => {
  it("imports and reproduces the documented sample", () => {
    const sample = [
      "0b. =Monkey; Squid Frog Beta; Ox Tiger Rabbit Dragon Horse Elephant Rat Ox Snake Dog Boar Frog; Dragon",
      "0a. =Fish; Rabbit Fish Alpha; Rat Snake Ram Monkey Rooster Dog Boar Tiger Ram Rooster Elephant Squid; Horse",
      "1b. Beta 5",
      "2a. Alpha 2",
      "3b. Beta *6",
      "4a. Alpha *1",
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
    ).toBe(`${betaRetreater!.animal} *6`);
    expect(
      formatMove(
        position,
        move("Beta", [
          { cardId: betaReserve!.id, from: "beta-reserve", to: "row-4" },
        ]),
      ),
    ).toBe(`${betaReserve!.animal} &4`);
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
    ).toBe(`${betaOther!.animal} 4, ${betaRetreater!.animal} *6`);
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

  it("exports Beta computer metadata in the documented format", () => {
    const timeline = [{ position: createFallbackGame(7_071), move: null }];
    const exported = serializeHistory(timeline, {
      exportDate: new Date(2026, 6, 24, 23, 59),
      computer: { player: "Beta", thinkingTimeSeconds: 10 },
    });

    expect(exported).toMatch(
      /^\/\/ Beta: Computer \(10 seconds of thinking time per ply\)\n\/\/ Alpha: Human\n\/\/ Export Date: 2026-07-24\n\n0b\./,
    );
    expect(parseHistory(exported, fallbackEngine)).toHaveLength(1);
  });

  it("exports Alpha computer metadata in Beta/Alpha order", () => {
    const exported = serializeHistory(
      [{ position: createFallbackGame(7_071), move: null }],
      {
        exportDate: new Date(2026, 0, 2),
        computer: { player: "Alpha", thinkingTimeSeconds: 1.25 },
      },
    );

    expect(exported).toMatch(
      /^\/\/ Beta: Human\n\/\/ Alpha: Computer \(1\.25 seconds of thinking time per ply\)\n\/\/ Export Date: 2026-01-02\n\n0b\./,
    );
  });

  it("exports only the date metadata for pass-and-play histories", () => {
    const exported = serializeHistory(
      [{ position: createFallbackGame(7_071), move: null }],
      { exportDate: new Date(2026, 10, 9) },
    );

    expect(exported).toMatch(/^\/\/ Export Date: 2026-11-09\n\n0b\./);
    expect(exported).not.toContain(": Human");
    expect(exported).not.toContain(": Computer");
  });

  it("formats Alpha and Beta snipe captures from the completed position", () => {
    const before = createFallbackGame(7_071);
    const completed = (
      winner: "Alpha" | "Beta",
      capturedOwner: "Alpha" | "Beta",
    ) => {
      const reserve = winner === "Alpha" ? "alpha-reserve" : "beta-reserve";
      const capturedId = `${capturedOwner.toLowerCase()}-snipe`;
      const locations = Object.fromEntries(
        Object.entries(before.locations).map(([location, cards]) => [
          location,
          cards.filter((card) => card.id !== capturedId),
        ]),
      ) as typeof before.locations;
      const captured = Object.values(before.locations)
        .flat()
        .find((card) => card.id === capturedId);
      expect(captured).toBeTruthy();
      locations[reserve] = [...locations[reserve], captured!];
      return { ...before, winner, locations };
    };
    const betaMove = fallbackLegalMoves(before)[0];

    const alphaWin = completed("Alpha", "Beta");
    const betaWin = completed("Beta", "Alpha");
    expect(snipeCaptureSuffix(before, alphaWin)).toBe("+#0");
    expect(snipeCaptureSuffix(before, betaWin)).toBe("-#0");
    expect(formatCompletedMove(before, betaMove, alphaWin)).toBe(
      `${formatMove(before, betaMove)}+#0`,
    );
    expect(formatCompletedMove(before, betaMove, betaWin)).toBe(
      `${formatMove(before, betaMove)}-#0`,
    );
    expect(
      formatCompletedMove(before, betaMove, { ...before, winner: "Beta" }),
    ).toBe(formatMove(before, betaMove));
  });

  it("annotates each animal-capturing subply independently", () => {
    const before = createFallbackGame(7_071);
    const movers = before.locations["row-2"].slice(0, 2);
    const victims = before.locations["row-5"].slice(0, 2);
    expect(movers).toHaveLength(2);
    expect(victims).toHaveLength(2);
    const withCaptured = (
      position: typeof before,
      victim: (typeof victims)[number],
    ) => ({
      ...position,
      locations: {
        ...position.locations,
        "row-5": position.locations["row-5"].filter(
          (card) => card.id !== victim.id,
        ),
        "alpha-reserve": [...position.locations["alpha-reserve"], victim],
      },
    });
    const afterFirst = withCaptured(before, victims[0]);
    const after = withCaptured(afterFirst, victims[1]);
    const twoStepMove = move("Alpha", [
      { cardId: movers[0].id, from: "row-2", to: "row-3" },
      { cardId: movers[1].id, from: "row-2", to: "row-3" },
    ]);

    expect(
      formatCompletedMove(before, twoStepMove, after, () => afterFirst),
    ).toBe(`${movers[0].animal} 3x, ${movers[1].animal} 3x`);

    const retreatCapture = move("Alpha", [
      { cardId: movers[0].id, from: "row-4", to: "row-3" },
    ]);
    expect(formatCompletedMove(before, retreatCapture, afterFirst)).toBe(
      `${movers[0].animal} *3x`,
    );

    const betaSnipe = before.locations["row-6"].find(
      (card) => card.id === "beta-snipe",
    )!;
    const winningAfter = {
      ...afterFirst,
      winner: "Alpha" as const,
      locations: {
        ...afterFirst.locations,
        "row-6": afterFirst.locations["row-6"].filter(
          (card) => card.id !== betaSnipe.id,
        ),
        "alpha-reserve": [
          ...afterFirst.locations["alpha-reserve"],
          betaSnipe,
        ],
      },
    };
    expect(
      formatCompletedMove(
        before,
        twoStepMove,
        winningAfter,
        () => afterFirst,
      ),
    ).toBe(`${movers[0].animal} 3x, ${movers[1].animal} 3+#0`);
    expect(formatCompletedMove(before, retreatCapture, winningAfter)).toBe(
      `${movers[0].animal} *3+#0`,
    );
  });

  it("exports terminal captures and accepts histories with omitted annotations", () => {
    const timeline = betaCaptureTimeline();
    const annotated = serializeHistory(timeline);
    expect(annotated.trimEnd()).toMatch(/-#0$/);
    expect(serializeHistory(parseHistory(annotated, fallbackEngine))).toBe(annotated);

    const unannotated = annotated.replace("-#0", "");
    expect(serializeHistory(parseHistory(unannotated, fallbackEngine))).toBe(annotated);
  });

  it("rejects lying terminal annotations", () => {
    const timeline = betaCaptureTimeline();
    const annotated = serializeHistory(timeline);
    expect(() =>
      parseHistory(annotated.replace("-#0", " -#0"), fallbackEngine),
    ).toThrow(/not a legal move/);
    expect(() =>
      parseHistory(annotated.replace("-#0", "+#0"), fallbackEngine),
    ).toThrow(/asserted result.*does not match/);

    const nonWinning = serializeHistory(timeline.slice(0, 2));
    expect(() =>
      parseHistory(nonWinning.replace(/\n$/, "-#0\n"), fallbackEngine),
    ).toThrow(/asserted result.*does not match/);

    expect(() =>
      parseHistory(annotated.replace("-#0", "x-#0"), fallbackEngine),
    ).toThrow(/asserted capture on subply 1 does not match/);
    expect(() =>
      parseHistory(annotated.replace("-#0", "x"), fallbackEngine),
    ).toThrow(/asserted capture on subply 1 does not match/);
  });

  it("rejects a capture annotation when that subply captures nothing", () => {
    const sample = [
      "0b. =Monkey; Squid Frog Beta; Ox Tiger Rabbit Dragon Horse Elephant Rat Ox Snake Dog Boar Frog; Dragon",
      "0a. =Fish; Rabbit Fish Alpha; Rat Snake Ram Monkey Rooster Dog Boar Tiger Ram Rooster Elephant Squid; Horse",
      "1b. Beta 5x",
      "",
    ].join("\n");

    expect(() => parseHistory(sample, fallbackEngine)).toThrow(
      /asserted capture on subply 1 does not match/,
    );
  });

  it("accepts omitted capture annotations and serializes them canonically", () => {
    const layout = [
      "0b. =Monkey; Squid Frog Beta; Ox Tiger Rabbit Dragon Horse Elephant Rat Ox Snake Dog Boar Frog; Dragon",
      "0a. =Fish; Rabbit Fish Alpha; Rat Snake Ram Monkey Rooster Dog Boar Tiger Ram Rooster Elephant Squid; Horse",
    ];
    const captureEngine = {
      legalMoves: (position: ReturnType<typeof createFallbackGame>) => {
        const ox = position.locations["row-5"].find(
          (card) => card.animal === "Ox",
        );
        return ox
          ? [
              move("Beta", [
                { cardId: ox.id, from: "row-5", to: "row-4" },
              ]),
            ]
          : [];
      },
      previewFirstStep: previewFallbackFirstStep,
      applyMove: (
        position: ReturnType<typeof createFallbackGame>,
        selected: TurnMove,
      ) => {
        const mover = position.locations["row-5"].find(
          (card) => card.id === selected.steps[0].cardId,
        )!;
        const captured = position.locations["row-4"][0];
        return {
          ...position,
          turn: "Alpha" as const,
          turnNumber: position.turnNumber + 1,
          locations: {
            ...position.locations,
            "row-5": position.locations["row-5"].filter(
              (card) => card.id !== mover.id,
            ),
            "row-4": [
              ...position.locations["row-4"].slice(1),
              mover,
            ],
            "beta-reserve": [
              ...position.locations["beta-reserve"],
              captured,
            ],
          },
        };
      },
    };
    const unannotated = [...layout, "1b. Ox 4", ""].join("\n");
    const annotated = [...layout, "1b. Ox 4x", ""].join("\n");

    expect(
      serializeHistory(parseHistory(unannotated, captureEngine)),
    ).toBe(annotated);
    expect(
      serializeHistory(parseHistory(annotated, captureEngine)),
    ).toBe(annotated);
    expect(() =>
      parseHistory(annotated.replace("4x", "4 &"), captureEngine),
    ).toThrow(/not a legal move/);
  });

  it("rejects the superseded postfix drop marker", () => {
    let position = createFallbackGame(7_071);
    const drop = fallbackLegalMoves(position).find((candidate) =>
      candidate.steps[0].from.includes("reserve"),
    );
    expect(drop).toBeTruthy();
    position = applyFallbackMove(position, drop!);
    const history = serializeHistory([
      { position: createFallbackGame(7_071), move: null },
      { position, move: drop! },
    ]);

    expect(() =>
      parseHistory(history.replace(/&(\d)/, "$1!"), fallbackEngine),
    ).toThrow(/not a legal move/);
  });

  it.each([
    ["uppercase player prefix", (history: string) => history.replace("1b.", "1B.")],
    ["Greek player prefix", (history: string) => history.replace("1b.", "1β.")],
    ["legacy retreat suffix", (history: string) => history.replace(/\*(\d)/, "$1*")],
    ["legacy retreat letter", (history: string) => history.replace(/\*(\d)/, "$1R")],
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

  it("rejects histories using paired ply numbering", () => {
    let position = createFallbackGame(7_071);
    const timeline: TimelineEntry[] = [{ position, move: null }];
    for (let index = 0; index < 2; index += 1) {
      const selected = fallbackLegalMoves(position)[0];
      position = applyFallbackMove(position, selected);
      timeline.push({ position, move: selected });
    }
    const paired = serializeHistory(timeline).replace("\n2a.", "\n1a.");

    expect(() => parseHistory(paired, fallbackEngine)).toThrow(
      'expected prefix "2a."',
    );
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
