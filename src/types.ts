import { Option, Result } from "rusty-ts";
import { GameState } from "./analyzer";

/**
 * Increment this when making breaking changes
 * to state interfaces to ensure seralized state
 * compatibility detection will continue to work.
 */
export const STATE_VERSION = 10;

export interface AppState {
  gameState: GameAnalyzer;
  ux: {
    selectedCardType: Option<CardType>;
    futureSubPlyStack: {
      plies: Ply[];
      pendingAnimalStep: Option<AnimalStep>;
    };
  };
}

export interface GameAnalyzer {
  getInitialState(): GameAnalyzer;
  getBoard(): Board;
  getPlies(): Ply[];
  getPendingAnimalStep(): Option<AnimalStep>;
  isGameOver(): boolean;
  getWinner(): Option<Player>;
  getTurn(): Player;
  getCardLocation(cardType: CardType): CardLocation;
  tryDrop(drop: Drop): Result<GameAnalyzer, IllegalGameStateUpdate>;
  tryAnimalStep(step: AnimalStep): Result<GameAnalyzer, IllegalGameStateUpdate>;
  tryUndoSubPly(): Result<
    { newState: GameAnalyzer; undone: SnipeStep | Drop | AnimalStep },
    IllegalGameStateUpdate
  >;
  tryPerform(atomic: Atomic): Result<GameAnalyzer, IllegalGameStateUpdate>;
  serialize(): string;
  toNodeKey(): string;
  setState(state: GameState): void;
  getStatesAfterPerformingOneAtomic(): GameState[];
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

export const allCardLocations = [
  CardLocation.AlphaReserve,
  CardLocation.Row1,
  CardLocation.Row2,
  CardLocation.Row3,
  CardLocation.Row4,
  CardLocation.Row5,
  CardLocation.Row6,
  CardLocation.BetaReserve,
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

export enum Player {
  Alpha = 0,
  Beta = 1,
}

// export type AnimalType =
//   | CardType.Mouse
//   | CardType.Ox
//   | CardType.Tiger
//   | CardType.Rabbit
//   | CardType.Dragon
//   | CardType.Snake
//   | CardType.Horse
//   | CardType.Ram
//   | CardType.Monkey
//   | CardType.Rooster
//   | CardType.Dog
//   | CardType.Boar
//   | CardType.Fish
//   | CardType.Elephant
//   | CardType.Squid
//   | CardType.Frog;

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
  dropped: CardType;
  destination: Row;
}

export interface TwoAnimalSteps {
  plyType: PlyType.TwoAnimalSteps;
  first: AnimalStep;
  second: AnimalStep;
}

export interface AnimalStep {
  moved: CardType;
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

export enum IllegalGameStateUpdate {}

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
  canRetreat: boolean;
}

export enum Element {
  Fire,
  Water,
  Earth,
  Air,
}

// export interface GameStateStruct {
//   stateVersion: typeof STATE_VERSION;

//   turn: Player;
//   alpha: Position;
//   beta: Position;
//   initialPositions: { alpha: Position; beta: Position };
//   plies: Ply[];

//   pendingAnimalStep: Option<AnimalStep>;
// }

// export interface Position {
//   reserve: Card[];
//   backRow: Card[];
//   frontRow: Card[];
// }

// export interface Card {
//   cardType: CardType;
//   instance: 0 | 1;
//   allegiance: Player;
// }

// export type Ply = SnipeStep | Drop | TwoAnimalSteps;

// export enum PlyType {
//   SnipeStep,
//   Drop,
//   TwoAnimalSteps,
// }

// export interface SnipeStep {
//   plyType: PlyType.SnipeStep;
//   destination: RowNumber;
// }

// export interface Drop {
//   plyType: PlyType.Drop;
//   dropped: CardType;
//   destination: RowNumber;
// }

// export interface TwoAnimalSteps {
//   plyType: PlyType.TwoAnimalSteps;
//   first: AnimalStep;
//   second: AnimalStep;
// }

// export interface AnimalStep {
//   moved: CardType;
//   destination: RowNumber;
// }

// export type RowNumber = 1 | 2 | 3 | 4 | 5 | 6;
