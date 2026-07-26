export type Player = "Alpha" | "Beta";
export type Strategy = "avocado" | "blueberry";
export type Location =
  | "alpha-reserve"
  | "beta-reserve"
  | `row-${1 | 2 | 3 | 4 | 5 | 6}`;

export interface Card {
  /** Value identity shared by interchangeable copies. */
  pieceKey: string;
  animal: string;
  owner: Player;
  isSnipe: boolean;
  canRetreat: boolean;
}

export interface LeadingAction {
  animal: string;
  direction: "advance" | "retreat";
  destination: number;
}

export interface Position {
  schemaVersion: 1;
  /** Canonical Core position identity, including an optional turn prefix. */
  positionKey: string;
  seed: number;
  turn: Player;
  turnNumber: number;
  winner: Player | null;
  leadingAction: LeadingAction | null;
  locations: Record<Location, Card[]>;
}

export interface Capture {
  animals: string[];
  /** Original owner of the captured snipe. */
  snipe: Player | null;
}

export interface MoveStep {
  pieceKey: string;
  animal: string;
  owner: Player;
  isSnipe: boolean;
  from: Location;
  to: Location;
  capture: Capture;
}

export interface TurnMove {
  id: string;
  /** The complete turn is legal only against this canonical position. */
  positionKey: string;
  player: Player;
  label: string;
  steps: MoveStep[];
  captures: Capture;
}

export interface AnalysisRequest {
  position: Position;
  timeLimitMs: number;
  requestId: number;
  strategy: Strategy;
  firstStep?: MoveStep;
}

export type LiveAnalysisRequest = AnalysisRequest;

export type EngineEvaluation =
  | { kind: "mate"; winner: Player; plies: number }
  | { kind: "estimate"; value: number };

export interface AnalysisResult {
  requestId: number;
  positionKey: string;
  bestMove: TurnMove;
  evaluation: EngineEvaluation;
  ticks: number;
  elapsedMs: number;
  recommendedLine: TurnMove[];
  strategy: Strategy;
  engineName: string;
}

export type LiveAnalysisUpdate = AnalysisResult;

export interface RulesEngine {
  readonly name: string;
  createGame(seed?: number): Position;
  canonicalizePosition(position: Position): Position;
  legalMoves(position: Position): TurnMove[];
  previewFirstStep(position: Position, step: MoveStep): Position;
  applyMove(position: Position, move: TurnMove): Position;
}

export interface ComputerAgent {
  chooseMove(
    request: AnalysisRequest,
    signal: AbortSignal,
  ): Promise<AnalysisResult>;
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

export function locationLabel(location: Location): string {
  if (location === "alpha-reserve") return "Alpha reserve";
  if (location === "beta-reserve") return "Beta reserve";
  return `Rank ${location.slice(-1)}`;
}

export function selectionKey(pieceKey: string, location: Location): string {
  return `${pieceKey}@${location}`;
}
