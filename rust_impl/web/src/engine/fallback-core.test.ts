import { describe, expect, it } from "vitest";
import {
  analyzeFallback,
  analyzeFallbackAtDepth,
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
  previewFallbackFirstStep,
} from "./fallback-core";

describe("engine adapter contract", () => {
  it("creates a reproducible complete board", () => {
    const first = createFallbackGame(42);
    const second = createFallbackGame(42);
    const cards = Object.values(first.locations).flat();

    expect(first).toEqual(second);
    expect(cards).toHaveLength(34);
    expect(cards.filter((card) => card.isSnipe)).toHaveLength(2);
    expect(first.turn).toBe("Beta");
  });

  it("generates applicable moves and advances a complete turn", () => {
    const position = createFallbackGame(19);
    const move = fallbackLegalMoves(position)[0];
    const next = applyFallbackMove(position, move);

    expect(next.turn).toBe("Alpha");
    expect(next.turnNumber).toBe(position.turnNumber + 1);
    expect(position.locations).not.toBe(next.locations);
  });

  it("previews a first animal step without advancing the turn", () => {
    const position = createFallbackGame(19);
    const move = fallbackLegalMoves(position).find(
      (candidate) => !candidate.steps[0].from.includes("reserve"),
    );
    expect(move).toBeDefined();

    const preview = previewFallbackFirstStep(position, move!.steps[0]);

    expect(preview.turn).toBe(position.turn);
    expect(preview.turnNumber).toBe(position.turnNumber);
    expect(
      preview.locations[move!.steps[0].to].some(
        (card) => card.id === move!.steps[0].cardId,
      ),
    ).toBe(true);
    expect(
      position.locations[move!.steps[0].from].some(
        (card) => card.id === move!.steps[0].cardId,
      ),
    ).toBe(true);
  });

  it("returns deterministic analysis in the browser protocol shape", () => {
    const position = createFallbackGame(7);
    const result = analyzeFallback(position, 81, 250);

    expect(result.requestId).toBe(81);
    expect(result.bestMove.player).toBe(position.turn);
    expect(result.nodes).toBeGreaterThan(0);
    expect(result.principalVariation.length).toBeGreaterThan(0);
  });

  it("returns a legal sequential live variation and honors the first-step constraint", () => {
    const position = createFallbackGame(31);
    const selected = fallbackLegalMoves(position)[3]!;
    const result = analyzeFallbackAtDepth(position, 82, 4, selected.steps[0]);

    expect(result.principalVariation.length).toBeGreaterThan(1);
    expect(result.principalVariation.length).toBeLessThanOrEqual(4);
    expect(result.principalVariation[0]).toEqual(result.bestMove);
    expect(result.principalVariation[0].steps[0]).toEqual(selected.steps[0]);

    let variationPosition = position;
    for (const move of result.principalVariation) {
      expect(fallbackLegalMoves(variationPosition).map((candidate) => candidate.id)).toContain(
        move.id,
      );
      variationPosition = applyFallbackMove(variationPosition, move);
    }
  });
});
