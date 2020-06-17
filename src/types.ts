import { Option, Result } from "rusty-ts";
import { MctsAnalyzerInternalData, MctsAnalyzer } from "./mcts";

/**
 * Increment this when making breaking changes
 * to state interfaces to ensure seralized state
 * compatibility detection will continue to work.
 */
export const STATE_VERSION = 14;

export interface AppState {
  gameState: GameState;
  ux: {
    selectedCardType: Option<CardType>;
    futureSubPlyStack: FutureSubPlyStack;
    analysisSuggestionDetailLevels: {
      [pointer: number]: SuggestionDetailLevel;
    };
  };
  mctsState: MctsState;
  thinkingTimeInMS: Option<number>;
  thinkingTimeInputValue: string;
}

export interface GameState {
  stateVersion: typeof STATE_VERSION;
  initialBoard: Int32Array;
  currentBoard: Int32Array;
  turn: Player;
  plies: number[];
  pendingAnimalStep: number;
}

export interface FutureSubPlyStack {
  stateVersion: typeof STATE_VERSION;
  atomics: Atomic[];
}

export type MctsState = MctsRunningState | MctsPausedState;

export interface MctsRunningState {
  isRunning: true;
  mostRecentSnapshot: Option<MctsAnalysisSnapshot>;
}

export interface MctsPausedState {
  isRunning: false;
  analyzer: MctsAnalyzer;
  expandedNodeIndexes: number[];
}

export interface MctsAnalysisSnapshot {
  currentStateValue: number;
  currentStateRollouts: number;

  bestAtomic: Atomic;
  bestAtomicValue: number;
  bestAtomicRollouts: number;
}

export enum SuggestionDetailLevel {
  None = "None",
  BestAction = "BestAction",
  AllActions = "AllActions",
}

export interface MctsService {
  updateGameState(state: GameState, optThinkingTimeInMS: Option<number>): void;
  pause(): void;
  resume(analyzer: MctsAnalyzer): void;

  onSnapshot(listener: (analysis: Option<MctsAnalysisSnapshot>) => void): void;
  onPause(listener: (analyzer: MctsAnalyzer) => void): void;
}

export interface GameStateAnalyzer {
  getInitialState(): GameState;
  getBoard(): Board;
  getPlies(): Ply[];
  getPendingAnimalStep(): Option<AnimalStep>;
  isGameOver(): boolean;
  getWinner(): Option<Player>;
  getTurn(): Player;
  getCardLocation(cardType: CardType): CardLocation;
  tryUndoSubPly(): Result<
    { newState: GameState; undone: SnipeStep | Drop | AnimalStep },
    IllegalGameStateUpdate
  >;
  tryPerform(atomic: Atomic): Result<GameState, IllegalGameStateUpdate>;
  serialize(): string;
  toNodeKey(): string;
  setState(state: GameState): void;
  getLegalAtomics(): Atomic[];
  forcePerform(atomic: Atomic): GameState;
}

export enum CardLocation {
  AlphaReserve = 0,
  Row1 = 1,
  Row2 = 2,
  Row3 = 3,
  Row4 = 4,
  Row5 = 5,
  Row6 = 6,
  BetaReserve = 7,
}

export const allCardLocations: CardLocation[] = [
  CardLocation.AlphaReserve,
  CardLocation.Row1,
  CardLocation.Row2,
  CardLocation.Row3,
  CardLocation.Row4,
  CardLocation.Row5,
  CardLocation.Row6,
  CardLocation.BetaReserve,
];

export const allRows: Row[] = [
  CardLocation.Row1,
  CardLocation.Row2,
  CardLocation.Row3,
  CardLocation.Row4,
  CardLocation.Row5,
  CardLocation.Row6,
];

export interface Board {
  [CardLocation.AlphaReserve]: Card[];
  [CardLocation.Row1]: Card[];
  [CardLocation.Row2]: Card[];
  [CardLocation.Row3]: Card[];
  [CardLocation.Row4]: Card[];
  [CardLocation.Row5]: Card[];
  [CardLocation.Row6]: Card[];
  [CardLocation.BetaReserve]: Card[];
}

export interface Card {
  cardType: CardType;
  allegiance: Player;
}

export enum CardType {
  Mouse1 = 0,
  Ox1 = 1,
  Tiger1 = 2,
  Rabbit1 = 3,
  Dragon1 = 4,
  Snake1 = 5,
  Horse1 = 6,
  Ram1 = 7,
  Monkey1 = 8,
  Rooster1 = 9,
  Dog1 = 10,
  Boar1 = 11,

  Fish1 = 12,
  Elephant1 = 13,
  Squid1 = 14,
  Frog1 = 15,

  Mouse2 = 16,
  Ox2 = 17,
  Tiger2 = 18,
  Rabbit2 = 19,
  Dragon2 = 20,
  Snake2 = 21,
  Horse2 = 22,
  Ram2 = 23,
  Monkey2 = 24,
  Rooster2 = 25,
  Dog2 = 26,
  Boar2 = 27,

  Fish2 = 28,
  Elephant2 = 29,
  Squid2 = 30,
  Frog2 = 31,

  AlphaSnipe = 32,
  BetaSnipe = 33,
}

export const allAnimalTypes: AnimalType[] = [
  CardType.Mouse1,
  CardType.Ox1,
  CardType.Tiger1,
  CardType.Rabbit1,
  CardType.Dragon1,
  CardType.Snake1,
  CardType.Horse1,
  CardType.Ram1,
  CardType.Monkey1,
  CardType.Rooster1,
  CardType.Dog1,
  CardType.Boar1,

  CardType.Fish1,
  CardType.Elephant1,
  CardType.Squid1,
  CardType.Frog1,

  CardType.Mouse2,
  CardType.Ox2,
  CardType.Tiger2,
  CardType.Rabbit2,
  CardType.Dragon2,
  CardType.Snake2,
  CardType.Horse2,
  CardType.Ram2,
  CardType.Monkey2,
  CardType.Rooster2,
  CardType.Dog2,
  CardType.Boar2,

  CardType.Fish2,
  CardType.Elephant2,
  CardType.Squid2,
  CardType.Frog2,
];

export type AnimalType = Exclude<CardType, SnipeType>;

export type SnipeType = CardType.AlphaSnipe | CardType.BetaSnipe;

export enum Player {
  Alpha = 0,
  Beta = 1,
}

export interface LegalRetreaterDrops {
  [Player.Alpha]: Row[];
  [Player.Beta]: Row[];
}

export const legalRetreaterDrops: LegalRetreaterDrops = {
  [Player.Alpha]: [
    CardLocation.Row1,
    CardLocation.Row2,
    CardLocation.Row3,
    CardLocation.Row4,
  ],
  [Player.Beta]: [
    CardLocation.Row3,
    CardLocation.Row4,
    CardLocation.Row5,
    CardLocation.Row6,
  ],
};

export type Ply = SnipeStep | Drop | TwoAnimalSteps;

export enum PlyType {
  SnipeStep,
  Drop,
  TwoAnimalSteps,
}

export interface SnipeStep {
  plyType: PlyType.SnipeStep;
  destination: Row;
}

export interface Drop {
  plyType: PlyType.Drop;
  dropped: AnimalType;
  destination: Row;
}

export interface TwoAnimalSteps {
  plyType: PlyType.TwoAnimalSteps;
  first: AnimalStep;
  second: AnimalStep;
}

export interface AnimalStep {
  moved: AnimalType;
  destination: Row;
}

export type Atomic = SnipeStep | Drop | AnimalStep;

export type Row =
  | CardLocation.Row1
  | CardLocation.Row2
  | CardLocation.Row3
  | CardLocation.Row4
  | CardLocation.Row5
  | CardLocation.Row6;

export enum IllegalGameStateUpdate {
  SnipeAlreadyCaptured,
  AlreadyMovedAnimal,
  StepDestinationOutOfRange,
  CannotEmptyRowWithoutImmediatelyWinning,
  DroppedAnimalNotInReserve,
  CannotEmptyReserve,
  CannotDropRetreaterOnEnemysBackTwoRows,
  MovedCardInReserve,
  NotYourAnimal,
  CannotMoveSameAnimalTwice,
  CannotCaptureOwnSnipeWithoutAlsoCapturingOpponents,

  NothingToUndo,
}

export interface StateSaver<T> {
  getState(): Option<T>;
  setState(state: T): void;
}

export interface CardMap<T> {
  [CardType.Mouse1]: T;
  [CardType.Ox1]: T;
  [CardType.Tiger1]: T;
  [CardType.Rabbit1]: T;
  [CardType.Dragon1]: T;
  [CardType.Snake1]: T;
  [CardType.Horse1]: T;
  [CardType.Ram1]: T;
  [CardType.Monkey1]: T;
  [CardType.Rooster1]: T;
  [CardType.Dog1]: T;
  [CardType.Boar1]: T;

  [CardType.Fish1]: T;
  [CardType.Elephant1]: T;
  [CardType.Squid1]: T;
  [CardType.Frog1]: T;

  [CardType.Mouse2]: T;
  [CardType.Ox2]: T;
  [CardType.Tiger2]: T;
  [CardType.Rabbit2]: T;
  [CardType.Dragon2]: T;
  [CardType.Snake2]: T;
  [CardType.Horse2]: T;
  [CardType.Ram2]: T;
  [CardType.Monkey2]: T;
  [CardType.Rooster2]: T;
  [CardType.Dog2]: T;
  [CardType.Boar2]: T;

  [CardType.Fish2]: T;
  [CardType.Elephant2]: T;
  [CardType.Squid2]: T;
  [CardType.Frog2]: T;

  [CardType.AlphaSnipe]: T;
  [CardType.BetaSnipe]: T;
}

export interface CardProperties {
  elements: Option<{ double: Element; single: Element }>;
  elementCounts: number;
  tripletShifts: [TripletShift, TripletShift];
  canRetreat: boolean;
}

export enum Element {
  Fire,
  Water,
  Earth,
  Air,
}

export enum ElementCount {
  F1 = 1 << 0,
  F2 = 1 << 1,
  F3 = 1 << 2,

  W1 = 1 << 3,
  W2 = 1 << 4,
  W3 = 1 << 5,

  E1 = 1 << 6,
  E2 = 1 << 7,
  E3 = 1 << 8,

  A1 = 1 << 9,
  A2 = 1 << 10,
  A3 = 1 << 11,
}

export enum TripletShift {
  Fire = 0,
  Water = 3,
  Earth = 6,
  Air = 9,
  None = 12,
}

export type MctsWorkerRequest =
  | UpdateGameStateRequest
  | PauseAnalyzerRequest
  | ResumeAnalyzerRequest;

export type MctsWorkerNotification =
  | LogNotification
  | UpdateSnapshotNotification
  | PauseAnalyzerResponse;

export enum MctsWorkerMessageType {
  UpdateGameStateRequest,
  PauseAnalyzerRequest,
  ResumeAnalyzerRequest,

  LogNotification,
  UpdateSnapshotNotification,
  PauseAnalyzerResponse,
}

export interface UpdateGameStateRequest {
  messageType: MctsWorkerMessageType.UpdateGameStateRequest;
  gameState: GameState;
  thinkingTimeInMS: number;
}

export interface PauseAnalyzerRequest {
  messageType: MctsWorkerMessageType.PauseAnalyzerRequest;
}

export interface ResumeAnalyzerRequest {
  messageType: MctsWorkerMessageType.ResumeAnalyzerRequest;
  internalData: MctsAnalyzerInternalData;
}

export interface LogNotification {
  messageType: MctsWorkerMessageType.LogNotification;
  data: unknown;
}

export interface UpdateSnapshotNotification {
  messageType: MctsWorkerMessageType.UpdateSnapshotNotification;
  optSnapshot: MctsAnalysisSnapshot | null;
}

export interface PauseAnalyzerResponse {
  messageType: MctsWorkerMessageType.PauseAnalyzerResponse;
  internalData: MctsAnalyzerInternalData;
}
