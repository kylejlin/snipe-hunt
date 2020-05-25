import { Option } from "rusty-ts";

/**
 * Increment this when making breaking changes
 * to state interfaces to ensure seralized state
 * compatibility detection will continue to work.
 */
export const STATE_VERSION = 10;

export interface AppState {
  gameState: GameState;
  ux: {
    selectedCardType: Option<CardType>;
    futurePlyStack: Ply[];
  };
}

export interface GameState {
  stateVersion: typeof STATE_VERSION;

  turn: Player;
  alpha: Position;
  beta: Position;
  initialPositions: { alpha: Position; beta: Position };
  plies: Ply[];

  pendingAnimalStep: Option<AnimalStep>;
}

export enum Player {
  Alpha,
  Beta,
}

export interface Position {
  reserve: Card[];
  backRow: Card[];
  frontRow: Card[];
}

export interface Card {
  cardType: CardType;
  instance: 0 | 1;
  allegiance: Player;
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

export type Ply = SnipeStep | Drop | TwoAnimalSteps;

export enum PlyType {
  SnipeStep,
  Drop,
  TwoAnimalSteps,
}

export interface SnipeStep {
  plyType: PlyType.SnipeStep;
  destination: RowNumber;
}

export interface Drop {
  plyType: PlyType.Drop;
  dropped: CardType;
  destination: RowNumber;
}

export interface TwoAnimalSteps {
  plyType: PlyType.TwoAnimalSteps;
  first: AnimalStep;
  second: AnimalStep;
}

export interface AnimalStep {
  moved: CardType;
  destination: RowNumber;
}

export type RowNumber = 1 | 2 | 3 | 4 | 5 | 6;

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

export interface CardMap<T> {
  [CardType.Snipe]: T;
  [CardType.Mouse]: T;
  [CardType.Ox]: T;
  [CardType.Tiger]: T;
  [CardType.Rabbit]: T;
  [CardType.Dragon]: T;
  [CardType.Snake]: T;
  [CardType.Horse]: T;
  [CardType.Ram]: T;
  [CardType.Monkey]: T;
  [CardType.Rooster]: T;
  [CardType.Dog]: T;
  [CardType.Boar]: T;
  [CardType.Fish]: T;
  [CardType.Elephant]: T;
  [CardType.Squid]: T;
  [CardType.Frog]: T;
}

export interface StateSaver<T> {
  getState(): Option<T>;
  setState(state: T): void;
}
