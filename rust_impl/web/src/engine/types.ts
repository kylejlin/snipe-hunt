export type Player = "Alpha" | "Beta";

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
  /** Earlier positions on the active timeline, oldest first. */
  history?: Position[];
  timeLimitMs: number;
  requestId: number;
}

export interface LiveAnalysisRequest {
  position: Position;
  /** Earlier positions on the active timeline, oldest first. */
  history?: Position[];
  maxDepth: number;
  requestId: number;
  firstStep?: MoveStep;
}

export interface CandidateLine {
  move: TurnMove;
  score: number;
}

export interface AnalysisResult {
  requestId: number;
  bestMove: TurnMove;
  score: number;
  depth: number;
  nodes: number;
  elapsedMs: number;
  principalVariation: string[];
  candidates: CandidateLine[];
  engineName: string;
}

export interface LiveAnalysisUpdate {
  requestId: number;
  bestMove: TurnMove;
  score: number;
  depth: number;
  principalVariation: TurnMove[];
}

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
