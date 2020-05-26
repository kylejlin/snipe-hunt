import { Option, Result } from "rusty-ts";

/**
 * Increment this when making breaking changes
 * to state interfaces to ensure seralized state
 * compatibility detection will continue to work.
 */
export const STATE_VERSION = 10;

export interface AppState {
  gameState: GameState;
  ux: {
    selectedCard: Option<Card>;
    futureSubPlyStack: {
      plies: Ply[];
      pendingAnimalStep: Option<AnimalStep>;
    };
  };
}

export interface GameState {
  getInitialState(): GameState;
  getBoard(): Board;
  getPlies(): Ply[];
  getPendingAnimalStep(): Option<AnimalStep>;
  isGameOver(): boolean;
  getTurn(): Player;
  getCardLocation(card: Omit<Card, "allegiance">): CardLocation;
  tryDrop(drop: Drop): Result<GameState, IllegalGameStateUpdate>;
  tryAnimalStep(step: AnimalStep): Result<GameState, IllegalGameStateUpdate>;
  tryUndoSubPly(): Result<
    { newState: GameState; undone: SnipeStep | Drop | AnimalStep },
    IllegalGameStateUpdate
  >;
  tryPerform(atomic: Atomic): Result<GameState, IllegalGameStateUpdate>;
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

export interface Board {
  [CardLocation.AlphaReserve]: AllegiantCard[];
  [CardLocation.Row1]: AllegiantCard[];
  [CardLocation.Row2]: AllegiantCard[];
  [CardLocation.Row3]: AllegiantCard[];
  [CardLocation.Row4]: AllegiantCard[];
  [CardLocation.Row5]: AllegiantCard[];
  [CardLocation.Row6]: AllegiantCard[];
  [CardLocation.BetaReserve]: AllegiantCard[];
}

export interface AllegiantCard extends Card {
  allegiance: Player;
}

export interface Card {
  cardType: CardType;
  instance: 0 | 1;
}

export enum CardType {
  Snipe = "Snipe",

  Mouse = "Mouse",
  Ox = "Ox",
  Tiger = "Tiger",
  Rabbit = "Rabbit",
  Dragon = "Dragon",
  Snake = "Snake",
  Horse = "Horse",
  Ram = "Ram",
  Monkey = "Monkey",
  Rooster = "Rooster",
  Dog = "Dog",
  Boar = "Boar",

  Fish = "Fish",
  Elephant = "Elephant",
  Squid = "Squid",
  Frog = "Frog",
}

export enum Player {
  Alpha,
  Beta,
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
  dropped: Card;
  destination: Row;
}

export interface TwoAnimalSteps {
  plyType: PlyType.TwoAnimalSteps;
  first: AnimalStep;
  second: AnimalStep;
}

export interface AnimalStep {
  moved: Card;
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

// export interface CardProperties {
//   elements: Option<{ double: Element; single: Element }>;
//   canRetreat: boolean;
// }

// export enum Element {
//   Fire,
//   Water,
//   Earth,
//   Air,
// }

// export interface CardMap<T> {
//   [CardType.Snipe]: T;
//   [CardType.Mouse]: T;
//   [CardType.Ox]: T;
//   [CardType.Tiger]: T;
//   [CardType.Rabbit]: T;
//   [CardType.Dragon]: T;
//   [CardType.Snake]: T;
//   [CardType.Horse]: T;
//   [CardType.Ram]: T;
//   [CardType.Monkey]: T;
//   [CardType.Rooster]: T;
//   [CardType.Dog]: T;
//   [CardType.Boar]: T;
//   [CardType.Fish]: T;
//   [CardType.Elephant]: T;
//   [CardType.Squid]: T;
//   [CardType.Frog]: T;
// }
