import {
  GameState,
  Ply,
  AnimalStep,
  Player,
  PlyType,
  SnipeStep,
  TwoAnimalSteps,
  Drop,
  CardType,
  Card,
  RowNumber,
} from "./types";
import { result, Result, Option } from "rusty-ts";

export interface GameAnalyzer {
  getBoardCards(): BoardCards;

  //   getState(): GameState;
  //   tryApplyPly(ply: Ply): Result<GameState, IllegalPly>;
  //   tryAnimalStep(step: AnimalStep): Result<GameState, IllegalPly>;
  //   getLegalPlies(): Ply[];
  //   getTurn(): Player;
  //   isGameOver(): boolean;
  //   getWinner(): Option<Player>;
}

export interface BoardCards {
  alphaReserve: Card[];
  betaReserve: Card[];
  rows: {
    1: Card[];
    2: Card[];
    3: Card[];
    4: Card[];
    5: Card[];
    6: Card[];
  };
}

export enum IllegalPly {
  SnipeAlreadyCaptured,
  AnimalStepIsPending,
  CannotEmptyRowWithoutCapturingEnemySnipe,
  StepDestinationOutOfRange,
}

export function getGameAnalyzer(state: GameState): GameAnalyzer {
  return {
    getState,
    tryApplyPly,
    tryAnimalStep,
    getLegalPlies,

    getTurn,
    isGameOver,
    getWinner,
  };

  function getState(): GameState {
    return state;
  }

  function tryApplyPly(ply: Ply): Result<GameState, IllegalPly> {
    switch (ply.plyType) {
      case PlyType.SnipeStep:
        return tryApplySnipeStep(ply);
      case PlyType.Drop:
        return tryApplyDrop(ply);
      case PlyType.TwoAnimalSteps:
        return tryApplyTwoAnimalSteps(ply);
    }
  }

  function tryApplySnipeStep(ply: SnipeStep): Result<GameState, IllegalPly> {
    if (isSnipeCaptured()) {
      return result.err(IllegalPly.SnipeAlreadyCaptured);
    }

    if (state.pendingAnimalStep.isSome()) {
      return result.err(IllegalPly.AnimalStepIsPending);
    }

    const snipe = getPlayerSnipe(state.turn);

    if (getRowWith(snipe).length === 1) {
      return result.err(IllegalPly.CannotEmptyRowWithoutCapturingEnemySnipe);
    }

    if (!isRowInRange(snipe, ply.destination)) {
      return result.err(IllegalPly.StepDestinationOutOfRange);
    }

    // TODO
  }

  function isSnipeCaptured(): boolean {}

  function getPlayerSnipe(player: Player): CardType.Snipe | CardType.BetaSnipe {
    switch (player) {
      case Player.Alpha:
        return CardType.Snipe;
      case Player.Beta:
        return CardType.BetaSnipe;
    }
  }

  function getRowWith(cardType: CardType): Option<Card[]> {}

  function isRowInRange(cardType: CardType, destination: RowNumber): boolean {}

  function tryApplyDrop(ply: Drop): Result<GameState, IllegalPly> {}

  function tryApplyTwoAnimalSteps(
    ply: TwoAnimalSteps
  ): Result<GameState, IllegalPly> {}

  function tryAnimalStep(step: AnimalStep): Result<GameState, IllegalPly> {}

  function getLegalPlies(): Ply[] {}

  function getTurn(): Player {}
  function isGameOver(): boolean {}
  function getWinner(): Option<Player> {}
}
