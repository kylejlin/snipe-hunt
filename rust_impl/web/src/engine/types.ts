export type Player = "Alpha" | "Beta";
export type Strategy = "avocado" | "blueberry";

export type Location = "alpha-reserve" | "beta-reserve" | `row-${1 | 2 | 3 | 4 | 5 | 6}`;

export interface Card {
  id: string;
  animal: string;
  owner: Player;
  isSnipe: boolean;
  canRetreat: boolean;
}

export interface Position {
  schemaVersion: 1;
  seed: number;
  turn: Player;
  turnNumber: number;
  winner: Player | null;
  locations: Record<Location, Card[]>;
}

export interface MoveStep {
  cardId: string;
  from: Location;
  to: Location;
}

export interface TurnMove {
  id: string;
  player: Player;
  label: string;
  steps: MoveStep[];
  captures: string[];
}

export interface AnalysisRequest {
  position: Position;
  timeLimitMs: number;
  requestId: number;
  strategy: Strategy;
  firstStep?: MoveStep;
}

export interface LiveAnalysisRequest {
  position: Position;
  timeLimitMs: number;
  requestId: number;
  strategy: Strategy;
  firstStep?: MoveStep;
}

export interface CandidateLine {
  move: TurnMove;
  score: number;
}

export interface AnalysisResult {
  requestId: number;
  bestMove: TurnMove;
  evaluation?: EngineEvaluation;
  ticks?: number;
  elapsedMs: number;
  recommendedLine?: TurnMove[];
  strategy?: Strategy;
  engineName: string;
  /** @deprecated Pre-0.32 compatibility fields. */
  score?: number;
  depth?: number;
  nodes?: number;
  principalVariation?: string[];
  candidates?: CandidateLine[];
}

export interface LiveAnalysisUpdate {
  requestId: number;
  bestMove: TurnMove;
  evaluation?: EngineEvaluation;
  ticks?: number;
  elapsedMs?: number;
  recommendedLine?: TurnMove[];
  strategy?: Strategy;
  engineName?: string;
  /** @deprecated Pre-0.32 compatibility fields. */
  score?: number;
  depth?: number;
  principalVariation?: TurnMove[];
}

export type EngineEvaluation =
  | { kind: "mate"; winner: Player; plies: number }
  | { kind: "estimate"; value: number };

export interface RulesEngine {
  readonly name: string;
  createGame(seed?: number): Position;
  legalMoves(position: Position): TurnMove[];
  previewFirstStep(position: Position, step: MoveStep): Position;
  applyMove(position: Position, move: TurnMove): Position;
}

export interface ComputerAgent {
  chooseMove(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult>;
  dispose(): void;
}

export interface LiveAnalyzer {
  analyze(
    request: LiveAnalysisRequest,
    onProgress: (update: LiveAnalysisUpdate) => void,
    signal: AbortSignal,
  ): Promise<LiveAnalysisUpdate>;
  dispose(): void;
}

export interface EngineServices {
  rules: RulesEngine;
  computerAgent: ComputerAgent;
  analyzer: LiveAnalyzer;
}

/** @deprecated Kept for compatibility with history-format test adapters. */
export interface EngineAdapter extends RulesEngine {
  analyze(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult>;
  dispose(): void;
}

export function otherPlayer(player: Player): Player {
  return player === "Alpha" ? "Beta" : "Alpha";
}

export function rowLocation(rank: number): Location {
  return `row-${Math.max(1, Math.min(6, rank)) as 1 | 2 | 3 | 4 | 5 | 6}`;
}

export function locationLabel(location: Location): string {
  if (location === "alpha-reserve") return "Alpha reserve";
  if (location === "beta-reserve") return "Beta reserve";
  return `Rank ${location.slice(-1)}`;
}
