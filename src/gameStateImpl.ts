import {
  GameState,
  Board,
  CardLocation,
  allCardLocations,
  CardType,
  Player,
  Ply,
  AnimalStep,
  Drop,
  IllegalGameStateUpdate,
  SnipeStep,
  Atomic,
} from "./types";
import { Option, Result } from "rusty-ts";

interface GameStateImpl extends GameState {}

interface GameStateData {
  initialBoard: Int32Array;
  currentBoard: Int32Array;
  turn: Player;
  plies: number[];
  pendingAnimalStep: number;
}

export interface GameStateImplUtils {
  fromString(s: string): Option<GameState>;
  fromBoard(board: Board): GameState;
}

export const gameStateImplUtils = getGameStateImplUtils();

enum Offset {
  AlphaAnimals = 0,
  BetaAnimals = 1,
  Snipes = 2,
}

enum Filter {
  LeastSixteenBits = 0b0000_0000_0000_0000_1111_1111_1111_1111,
}

function getGameStateImplUtils(): GameStateImplUtils {
  return { fromString, fromBoard };

  function fromString(s: string): Option<GameStateImpl> {}

  function fromBoard(board: Board): GameStateImpl {
    const initialBoard = new Int32Array(24);

    for (const location of allCardLocations) {
      const cards = board[location];
      for (const card of cards) {
        if (
          card.cardType === CardType.AlphaSnipe ||
          card.cardType === CardType.BetaSnipe
        ) {
          const cardSet = card.cardType === CardType.AlphaSnipe ? 1 : 1 << 1;
          const offset = Offset.Snipes;
          initialBoard[location + offset] |= cardSet;
        } else {
          const cardSet = 1 << card.cardType;
          const offset =
            card.allegiance === Player.Alpha
              ? Offset.AlphaAnimals
              : Offset.BetaAnimals;
          initialBoard[location + offset] |= cardSet;
        }
      }
    }

    const data: GameStateData = {
      initialBoard,
      currentBoard: initialBoard,
      turn: Player.Beta,
      plies: [],
      pendingAnimalStep: 0,
    };

    return getGameStateFromData(data);
  }

  function getGameStateFromData(data: GameStateData): GameStateImpl {
    return {
      getInitialState,
      getBoard,
      getPlies,
      getPendingAnimalStep,
      isGameOver,
      getTurn,
      getCardLocation,
      tryDrop,
      tryAnimalStep,
      tryUndoSubPly,
      tryPerform,
      serialize,
      toNodeKey,
    };

    function getInitialState(): GameState {}

    function getBoard(): Board {}

    function getPlies(): Ply[] {}

    function getPendingAnimalStep(): Option<AnimalStep> {}

    function isGameOver(): boolean {}

    function getTurn(): Player {}

    function getCardLocation(cardType: CardType): CardLocation {}

    function tryDrop(drop: Drop): Result<GameState, IllegalGameStateUpdate> {}

    function tryAnimalStep(
      step: AnimalStep
    ): Result<GameState, IllegalGameStateUpdate> {}

    function tryUndoSubPly(): Result<
      { newState: GameState; undone: SnipeStep | Drop | AnimalStep },
      IllegalGameStateUpdate
    > {}

    function tryPerform(
      atomic: Atomic
    ): Result<GameState, IllegalGameStateUpdate> {}

    function serialize(): string {}

    function toNodeKey(): string {
      const { currentBoard } = data;
      return String.fromCharCode(
        currentBoard[0] & Filter.LeastSixteenBits,
        currentBoard[0] >>> 16,
        currentBoard[1] & Filter.LeastSixteenBits,
        currentBoard[1] >>> 16,
        currentBoard[2] & Filter.LeastSixteenBits,
        currentBoard[2] >>> 16,
        currentBoard[3] & Filter.LeastSixteenBits,
        currentBoard[3] >>> 16,
        currentBoard[4] & Filter.LeastSixteenBits,
        currentBoard[4] >>> 16,
        currentBoard[5] & Filter.LeastSixteenBits,
        currentBoard[5] >>> 16,
        currentBoard[6] & Filter.LeastSixteenBits,
        currentBoard[6] >>> 16,
        currentBoard[7] & Filter.LeastSixteenBits,
        currentBoard[7] >>> 16,
        currentBoard[8] & Filter.LeastSixteenBits,
        currentBoard[8] >>> 16,
        currentBoard[9] & Filter.LeastSixteenBits,
        currentBoard[9] >>> 16,
        currentBoard[10] & Filter.LeastSixteenBits,
        currentBoard[10] >>> 16,
        currentBoard[11] & Filter.LeastSixteenBits,
        currentBoard[11] >>> 16,
        currentBoard[12] & Filter.LeastSixteenBits,
        currentBoard[12] >>> 16,
        currentBoard[13] & Filter.LeastSixteenBits,
        currentBoard[13] >>> 16,
        currentBoard[14] & Filter.LeastSixteenBits,
        currentBoard[14] >>> 16,
        currentBoard[15] & Filter.LeastSixteenBits,
        currentBoard[15] >>> 16,
        currentBoard[16] & Filter.LeastSixteenBits,
        currentBoard[16] >>> 16,
        currentBoard[17] & Filter.LeastSixteenBits,
        currentBoard[17] >>> 16,
        currentBoard[18] & Filter.LeastSixteenBits,
        currentBoard[18] >>> 16,
        currentBoard[19] & Filter.LeastSixteenBits,
        currentBoard[19] >>> 16,
        currentBoard[20] & Filter.LeastSixteenBits,
        currentBoard[20] >>> 16,
        currentBoard[21] & Filter.LeastSixteenBits,
        currentBoard[21] >>> 16,
        currentBoard[22] & Filter.LeastSixteenBits,
        currentBoard[22] >>> 16,
        currentBoard[23] & Filter.LeastSixteenBits,
        currentBoard[23] >>> 16,

        (data.turn << 15) | data.pendingAnimalStep
      );
    }
  }
}
