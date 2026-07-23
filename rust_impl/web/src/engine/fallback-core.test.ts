import { describe, expect, it } from "vitest";
import {
  analyzeFallback,
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
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

  it("returns deterministic analysis in the browser protocol shape", () => {
    const position = createFallbackGame(7);
    const result = analyzeFallback(position, 81, 250);

    expect(result.requestId).toBe(81);
    expect(result.bestMove.player).toBe(position.turn);
    expect(result.nodes).toBeGreaterThan(0);
    expect(result.principalVariation.length).toBeGreaterThan(0);
  });
});
