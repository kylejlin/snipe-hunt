import type { RulesEngine } from "../engine/types";
import {
  activeTimeline,
  newGame,
  type AlternativeLine,
  type GameState,
  type TimelineEntry,
} from "./game-state";

export const STORAGE_KEY = "snipe-hunt.web2.game";

export function restoreGame(
  serialized: string | null,
  engine: RulesEngine,
): GameState {
  if (!serialized) return newGame(engine.createGame());
  try {
    const parsed = JSON.parse(serialized) as unknown;
    assertGameState(parsed);
    validateTimeline(parsed.timeline, engine);
    validateAlternative(parsed.alternativeLine, parsed.timeline, engine);
    const displayed = activeTimeline(parsed);
    if (
      !Number.isInteger(parsed.cursor) ||
      parsed.cursor < 0 ||
      parsed.cursor >= displayed.length
    ) {
      throw new Error("invalid cursor");
    }
    if (parsed.subply) {
      const committedStep = displayed[parsed.cursor + 1]?.move?.steps[0] ?? null;
      if (Boolean(committedStep) === Boolean(parsed.draftStep)) {
        throw new Error("ambiguous turn prefix");
      }
      const step = committedStep ?? parsed.draftStep;
      if (!step) throw new Error("missing turn prefix");
      engine.previewFirstStep(displayed[parsed.cursor].position, step);
    }
    return parsed;
  } catch {
    return newGame(engine.createGame());
  }
}

export function saveGame(game: GameState): string {
  return JSON.stringify(game);
}

function validateTimeline(
  timeline: TimelineEntry[],
  engine: RulesEngine,
): void {
  if (!Array.isArray(timeline) || timeline.length === 0) {
    throw new Error("empty timeline");
  }
  if (timeline[0].move !== null || timeline[0].position.turnNumber !== 1) {
    throw new Error("invalid initial timeline entry");
  }
  engine.legalMoves(timeline[0].position);
  for (let index = 1; index < timeline.length; index += 1) {
    const previous = timeline[index - 1].position;
    const entry = timeline[index];
    if (!entry.move) throw new Error("missing timeline move");
    const applied = engine.applyMove(previous, entry.move);
    if (
      applied.positionKey !== entry.position.positionKey ||
      applied.seed !== entry.position.seed ||
      applied.turnNumber !== entry.position.turnNumber
    ) {
      throw new Error("timeline position does not match its move");
    }
    // This also makes Core verify that the serialized position contents
    // actually match the claimed canonical key, including the terminal entry.
    engine.legalMoves(entry.position);
  }
}

function validateAlternative(
  alternative: AlternativeLine | null,
  timeline: TimelineEntry[],
  engine: RulesEngine,
): void {
  if (!alternative) return;
  if (
    !Number.isInteger(alternative.divergenceIndex) ||
    alternative.divergenceIndex < 0 ||
    alternative.divergenceIndex >= timeline.length
  ) {
    throw new Error("invalid alternative divergence");
  }
  let position = timeline[alternative.divergenceIndex].position;
  for (const entry of alternative.entries) {
    if (!entry.move) throw new Error("missing alternative move");
    position = engine.applyMove(position, entry.move);
    if (
      position.positionKey !== entry.position.positionKey ||
      position.seed !== entry.position.seed ||
      position.turnNumber !== entry.position.turnNumber
    ) {
      throw new Error("alternative position does not match its move");
    }
    engine.legalMoves(entry.position);
  }
}

function assertGameState(value: unknown): asserts value is GameState {
  if (!value || typeof value !== "object") throw new Error("invalid game");
  const game = value as Partial<GameState>;
  if (game.schemaVersion !== 1) throw new Error("unsupported schema");
  if (!Array.isArray(game.timeline)) throw new Error("invalid timeline");
  if (game.activeLine !== "actual" && game.activeLine !== "alternative") {
    throw new Error("invalid active line");
  }
  if (game.activeLine === "alternative" && !game.alternativeLine) {
    throw new Error("missing alternative line");
  }
  if (
    game.gameMode !== "computer-alpha" &&
    game.gameMode !== "computer-beta" &&
    game.gameMode !== "pass-and-play"
  ) {
    throw new Error("invalid game mode");
  }
  if (game.strategy !== "avocado" && game.strategy !== "blueberry") {
    throw new Error("invalid strategy");
  }
  if (typeof game.analysisEnabled !== "boolean") {
    throw new Error("invalid analysis setting");
  }
  assertSeconds(game.thinkingTimeSeconds, "thinking time");
  assertSeconds(game.analysisTimeSeconds, "analysis time");
  if (typeof game.subply !== "boolean") throw new Error("invalid subply");
  if (game.draftStep && !game.subply) throw new Error("orphaned draft");
}

function assertSeconds(value: unknown, label: string): asserts value is number {
  if (
    typeof value !== "number" ||
    !Number.isFinite(value) ||
    value < 0.25 ||
    value > 120
  ) {
    throw new Error(`invalid ${label}`);
  }
}
