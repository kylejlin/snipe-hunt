import type {
  MoveStep,
  Position,
  Strategy,
  TurnMove,
} from "../engine/types";

export type GameMode =
  | "computer-alpha"
  | "computer-beta"
  | "pass-and-play";
export type ActiveLine = "actual" | "alternative";

export interface TimelineEntry {
  position: Position;
  move: TurnMove | null;
}

export interface AlternativeLine {
  divergenceIndex: number;
  entries: TimelineEntry[];
}

export interface GameState {
  schemaVersion: 1;
  timeline: TimelineEntry[];
  alternativeLine: AlternativeLine | null;
  activeLine: ActiveLine;
  cursor: number;
  subply: boolean;
  draftStep: MoveStep | null;
  gameMode: GameMode;
  thinkingTimeSeconds: number;
  strategy: Strategy;
  analysisEnabled: boolean;
  analysisTimeSeconds: number;
}

export type GameAction =
  | { type: "replace"; state: GameState }
  | {
      type: "commit";
      basePositionKey: string;
      position: Position;
      move: TurnMove;
    }
  | { type: "draft"; step: MoveStep }
  | { type: "navigate"; cursor: number; subply: boolean; line: ActiveLine }
  | { type: "clear-draft" }
  | {
      type: "settings";
      values: Partial<
        Pick<
          GameState,
          | "gameMode"
          | "thinkingTimeSeconds"
          | "strategy"
          | "analysisEnabled"
          | "analysisTimeSeconds"
        >
      >;
    };

export function newGame(position: Position): GameState {
  return {
    schemaVersion: 1,
    timeline: [{ position, move: null }],
    alternativeLine: null,
    activeLine: "actual",
    cursor: 0,
    subply: false,
    draftStep: null,
    gameMode: "computer-beta",
    thinkingTimeSeconds: 5,
    strategy: "cherry",
    analysisEnabled: false,
    analysisTimeSeconds: 2,
  };
}

export function activeTimeline(game: GameState): TimelineEntry[] {
  if (game.activeLine !== "alternative" || !game.alternativeLine) {
    return game.timeline;
  }
  return [
    ...game.timeline.slice(0, game.alternativeLine.divergenceIndex + 1),
    ...game.alternativeLine.entries,
  ];
}

export function gameReducer(game: GameState, action: GameAction): GameState {
  switch (action.type) {
    case "replace":
      return action.state;
    case "settings":
      return { ...game, ...action.values };
    case "clear-draft":
      return { ...game, subply: false, draftStep: null };
    case "navigate":
      return {
        ...game,
        activeLine: action.line,
        cursor: action.cursor,
        subply: action.subply,
      };
    case "draft": {
      if (game.activeLine === "alternative" && game.alternativeLine) {
        const timeline = activeTimeline(game);
        const oldDivergence = game.alternativeLine.divergenceIndex;
        return {
          ...game,
          alternativeLine: {
            divergenceIndex:
              game.cursor >= oldDivergence ? oldDivergence : game.cursor,
            entries:
              game.cursor >= oldDivergence
                ? timeline.slice(oldDivergence + 1, game.cursor + 1)
                : [],
          },
          subply: true,
          draftStep: action.step,
        };
      }
      return {
        ...game,
        timeline: game.timeline.slice(0, game.cursor + 1),
        alternativeLine:
          game.alternativeLine &&
          game.cursor >= game.alternativeLine.divergenceIndex
            ? game.alternativeLine
            : null,
        activeLine: "actual",
        subply: true,
        draftStep: action.step,
      };
    }
    case "commit": {
      const timeline = activeTimeline(game);
      if (timeline[game.cursor]?.position.positionKey !== action.basePositionKey) {
        return game;
      }
      const entry = { position: action.position, move: action.move };
      if (game.activeLine === "alternative" && game.alternativeLine) {
        const oldDivergence = game.alternativeLine.divergenceIndex;
        const divergenceIndex =
          game.cursor >= oldDivergence ? oldDivergence : game.cursor;
        const entries =
          game.cursor >= oldDivergence
            ? timeline
                .slice(oldDivergence + 1, game.cursor + 1)
                .concat(entry)
            : [entry];
        return {
          ...game,
          alternativeLine: { divergenceIndex, entries },
          cursor: game.cursor + 1,
          subply: false,
          draftStep: null,
        };
      }
      const nextTimeline = game.timeline
        .slice(0, game.cursor + 1)
        .concat(entry);
      return {
        ...game,
        timeline: nextTimeline,
        alternativeLine:
          game.alternativeLine &&
          game.cursor >= game.alternativeLine.divergenceIndex
            ? game.alternativeLine
            : null,
        activeLine: "actual",
        cursor: nextTimeline.length - 1,
        subply: false,
        draftStep: null,
      };
    }
  }
}

export function computerControls(mode: GameMode, turn: Position["turn"]): boolean {
  return (
    (mode === "computer-alpha" && turn === "Alpha") ||
    (mode === "computer-beta" && turn === "Beta")
  );
}
