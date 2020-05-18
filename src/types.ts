import { Option } from "rusty-ts";

/**
 * Increment this when making breaking changes
 * to state interfaces to ensure seralized state
 * compatibility detection will continue to work.
 */
export const STATE_VERSION = 9;

export interface AppState {
  stateVersion: typeof STATE_VERSION;
  gameState: GameState;
  selectedCard: Option<CardType>;
}

export type GameState = Readonly<MutGameState>;

export interface MutGameState {
  turn: Player;
  alpha: Position;
  beta: Position;
  initialPositions: { alpha: Position; beta: Position };
  plies: Ply[];
  futurePlyStack: Ply[];
  pendingSubPly: Option<SubPly>;
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

export type Card = Readonly<MutCard>;

export interface MutCard {
  cardType: CardType;
  allegiance: Player;
  isPromoted: boolean;
}

export enum CardType {
  AlphaSnipe,
  BetaSnipe,

  Mouse,
  Ox,
  Tiger,
  Rabbit,
  Dragon,
  Snake,
  Horse,
  Ram,
  Monkey,
  Rooster,
  Dog,
  Boar,

  Fish,
  Elephant,
  Squid,
  Frog,
}

export type Ply = DemoteMove | MovePromote | Drop;

export enum PlyType {
  DemoteMove,
  MovePromote,
  Drop,
}

export interface DemoteMove {
  plyType: PlyType.DemoteMove;
  demoted: CardType;
  moved: CardType;
  destination: Row;
  captures: CardType[];
}

export interface MovePromote {
  plyType: PlyType.MovePromote;
  moved: CardType;
  destination: Row;
  captures: CardType[];
  promoted: CardType;
}

export interface Drop {
  plyType: PlyType.Drop;
  dropped: CardType;
  destination: Row;
}

export type SubPly = DemoteSubPly | MoveSubPly;

export enum SubPlyType {
  Demote,
  Move,
}

export interface DemoteSubPly {
  subPlyType: SubPlyType.Demote;
  demoted: CardType;
}

export interface MoveSubPly {
  subPlyType: SubPlyType.Move;
  moved: CardType;
  destination: Row;
  captures: CardType[];
}

export type Row = 1 | 2 | 3 | 4;

export interface StateSaver<T> {
  getState(): Option<T>;
  setState(state: T): void;
}

export interface CardPropertyDefinition {
  elements: Option<{
    unpromoted: { double: Element; single: Element };
    promoted: { double: Element; single: Element };
  }>;
  canUnpromotedMoveBackward: boolean;
  canPromotedMoveBackward: boolean;
}

export interface CardMap<T> {
  [CardType.AlphaSnipe]: T;
  [CardType.BetaSnipe]: T;
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

export type CardPropertyMap = CardMap<CardPropertyDefinition>;

export enum Element {
  Fire,
  Water,
  Earth,
  Air,
}
