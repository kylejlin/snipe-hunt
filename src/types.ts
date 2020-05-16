import { Option } from "rusty-ts";

/**
 * Increment this when making breaking changes
 * to state interfaces to ensure seralized state
 * compatibility detection will continue to work.
 */
export const STATE_VERSION = 7;

export interface AppState {
  stateVersion: typeof STATE_VERSION;
  gameState: GameState;
  selectedCard: Option<CardType>;
}

export interface GameState {
  turn: Player;
  alpha: Position;
  beta: Position;
  initialPositions: { alpha: Position; beta: Position };
  plies: Ply[];
  futurePlies: Ply[];
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

export interface Card {
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
  drop: CardType;
  destination: Row;
}

export type SubPly = Demote | Move;

export enum SubPlyType {
  Demote,
  Move,
}

export interface Demote {
  subPlyType: SubPlyType.Demote;
  demoted: CardType;
}

export interface Move {
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

export interface CardPropertyMap {
  [CardType.AlphaSnipe]: CardPropertyDefinition;
  [CardType.BetaSnipe]: CardPropertyDefinition;
  [CardType.Mouse]: CardPropertyDefinition;
  [CardType.Ox]: CardPropertyDefinition;
  [CardType.Tiger]: CardPropertyDefinition;
  [CardType.Rabbit]: CardPropertyDefinition;
  [CardType.Dragon]: CardPropertyDefinition;
  [CardType.Snake]: CardPropertyDefinition;
  [CardType.Horse]: CardPropertyDefinition;
  [CardType.Ram]: CardPropertyDefinition;
  [CardType.Monkey]: CardPropertyDefinition;
  [CardType.Rooster]: CardPropertyDefinition;
  [CardType.Dog]: CardPropertyDefinition;
  [CardType.Boar]: CardPropertyDefinition;
  [CardType.Fish]: CardPropertyDefinition;
  [CardType.Elephant]: CardPropertyDefinition;
  [CardType.Squid]: CardPropertyDefinition;
  [CardType.Frog]: CardPropertyDefinition;
}

export enum Element {
  Fire,
  Water,
  Earth,
  Air,
}
