import { Option, Result } from "rusty-ts";
import { Filter } from "./bitwiseUtils";
import {
  AnimalStep,
  Atomic,
  Board,
  CardLocation,
  CardType,
  Drop,
  GameAnalyzer,
  GameState,
  IllegalGameStateUpdate,
  Player,
  Ply,
  SnipeStep,
} from "./types";

export function getAnalyzer(initState: GameState): GameAnalyzer {
  let state = initState;

  return {
    getInitialState,
    getBoard,
    getPlies,
    getPendingAnimalStep,
    isGameOver,
    getWinner,
    getTurn,
    getCardLocation,
    tryDrop,
    tryAnimalStep,
    tryUndoSubPly,
    tryPerform,
    serialize,
    toNodeKey,
    setState,
    getStatesAfterPerformingOneAtomic,
  };

  function getInitialState(): GameState {}

  function getBoard(): Board {}

  function getPlies(): Ply[] {}

  function getPendingAnimalStep(): Option<AnimalStep> {}

  function isGameOver(): boolean {}

  function getWinner(): Option<Player> {}

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
    const { currentBoard } = state;
    return String.fromCharCode(
      // Animals
      currentBoard[0] & Filter.LeastSixteenBits,
      currentBoard[0] >>> 16,
      currentBoard[1] & Filter.LeastSixteenBits,
      currentBoard[1] >>> 16,
      currentBoard[3] & Filter.LeastSixteenBits,
      currentBoard[3] >>> 16,
      currentBoard[4] & Filter.LeastSixteenBits,
      currentBoard[4] >>> 16,
      currentBoard[6] & Filter.LeastSixteenBits,
      currentBoard[6] >>> 16,
      currentBoard[7] & Filter.LeastSixteenBits,
      currentBoard[7] >>> 16,
      currentBoard[9] & Filter.LeastSixteenBits,
      currentBoard[9] >>> 16,
      currentBoard[10] & Filter.LeastSixteenBits,
      currentBoard[10] >>> 16,
      currentBoard[12] & Filter.LeastSixteenBits,
      currentBoard[12] >>> 16,
      currentBoard[13] & Filter.LeastSixteenBits,
      currentBoard[13] >>> 16,
      currentBoard[15] & Filter.LeastSixteenBits,
      currentBoard[15] >>> 16,
      currentBoard[16] & Filter.LeastSixteenBits,
      currentBoard[16] >>> 16,
      currentBoard[18] & Filter.LeastSixteenBits,
      currentBoard[18] >>> 16,
      currentBoard[19] & Filter.LeastSixteenBits,
      currentBoard[19] >>> 16,
      currentBoard[21] & Filter.LeastSixteenBits,
      currentBoard[21] >>> 16,
      currentBoard[22] & Filter.LeastSixteenBits,
      currentBoard[22] >>> 16,

      // Snipes
      currentBoard[2] |
        (currentBoard[5] << 2) |
        (currentBoard[8] << 4) |
        (currentBoard[11] << 6) |
        (currentBoard[14] << 8) |
        (currentBoard[17] << 10) |
        (currentBoard[20] << 12) |
        (currentBoard[23] << 14),

      // Turn and animal step
      (state.turn << 1) | (state.pendingAnimalStep & Filter.LeastBit)
    );
  }

  function setState(newState: GameState): void {
    state = newState;
  }

  function getStatesAfterPerformingOneAtomic(): GameState[] {}
}
