import { Option, Result, option } from "rusty-ts";
import { Filter, Offset, PlyTag } from "./bitwiseUtils";
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
  allCardLocations,
  allAnimalTypes,
  Row,
  PlyType,
  SnipeType,
  AnimalType,
  allRows,
  legalRetreaterDrops,
} from "./types";
import {
  opponentOf,
  snipeOf,
  oneRowForward,
  isRow,
  oneRowBackward,
  canRetreat,
} from "./gameUtil";
import { cardProperties } from "./cardMaps";

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
    tryUndoSubPly,
    tryPerform,
    serialize,
    toNodeKey,
    setState,
    getStatesAfterPerformingOneAtomic,
  };

  function getInitialState(): GameState {
    return {
      initialBoard: state.initialBoard,
      currentBoard: state.initialBoard,
      turn: Player.Beta,
      plies: [],
      pendingAnimalStep: 0,
    };
  }

  function getBoard(): Board {
    const board: Board = {
      [CardLocation.AlphaReserve]: [],
      [CardLocation.Row1]: [],
      [CardLocation.Row2]: [],
      [CardLocation.Row3]: [],
      [CardLocation.Row4]: [],
      [CardLocation.Row5]: [],
      [CardLocation.Row6]: [],
      [CardLocation.BetaReserve]: [],
    };

    for (const location of allCardLocations) {
      const cards = board[location];

      const alphaAnimals =
        state.currentBoard[location * 3 + Offset.AlphaAnimals];
      const betaAnimals = state.currentBoard[location * 3 + Offset.BetaAnimals];

      allAnimalTypes.forEach((animalType) => {
        if (((1 << animalType) & alphaAnimals) !== 0) {
          cards.push({ cardType: animalType, allegiance: Player.Alpha });
        }

        if (((1 << animalType) & betaAnimals) !== 0) {
          cards.push({ cardType: animalType, allegiance: Player.Beta });
        }
      });

      const snipes = state.currentBoard[location * 3 + Offset.Snipes];

      if (((1 << Player.Alpha) & snipes) !== 0) {
        cards.push({ cardType: CardType.AlphaSnipe, allegiance: Player.Alpha });
      }
      if (((1 << Player.Beta) & snipes) !== 0) {
        cards.push({ cardType: CardType.BetaSnipe, allegiance: Player.Beta });
      }
    }

    return board;
  }

  function getPlies(): Ply[] {
    return state.plies.map(decodePly);
  }

  function decodePly(ply: number): Ply {
    const tag = (ply & Filter.LeastThreeBits) as PlyTag;
    switch (tag) {
      case PlyTag.SnipeStep: {
        const destination = ((ply >>> 3) & Filter.LeastThreeBits) as Row;
        return { plyType: PlyType.SnipeStep, destination };
      }

      case PlyTag.Drop: {
        const cardType = ((ply >>> 3) & Filter.LeastThreeBits) as AnimalType;
        const destination = ((ply >>> 8) & Filter.LeastThreeBits) as Row;
        return {
          plyType: PlyType.Drop,
          dropped: cardType,
          destination,
        };
      }

      case PlyTag.TwoAnimalSteps: {
        const firstCardType = ((ply >>> 3) &
          Filter.LeastFiveBits) as AnimalType;
        const firstDestination = ((ply >>> 8) & Filter.LeastThreeBits) as Row;
        const secondCardType = ((ply >>> 11) &
          Filter.LeastFiveBits) as AnimalType;
        const secondDestination = ((ply >>> 16) & Filter.LeastThreeBits) as Row;
        return {
          plyType: PlyType.TwoAnimalSteps,
          first: { moved: firstCardType, destination: firstDestination },
          second: { moved: secondCardType, destination: secondDestination },
        };
      }
    }
  }

  function getPendingAnimalStep(): Option<AnimalStep> {
    const { pendingAnimalStep } = state;

    if (pendingAnimalStep === 0) {
      return option.none();
    }

    const cardType = ((pendingAnimalStep >>> 3) &
      Filter.LeastFiveBits) as AnimalType;
    const destination = ((pendingAnimalStep >>> 8) &
      Filter.LeastThreeBits) as Row;
    return option.some({
      moved: cardType,
      destination,
    });
  }

  function isGameOver(): boolean {
    return getWinner().isSome();
  }

  function getWinner(): Option<Player> {
    if (isAlphaSnipeCapturedByBeta()) {
      return option.some(Player.Beta);
    }

    if (isBetaSnipeCapturedByAlpha()) {
      return option.some(Player.Alpha);
    }

    if (getLegalAtomics().length === 0) {
      return option.some(opponentOf(state.turn));
    }

    return option.none();
  }

  function isAlphaSnipeCapturedByBeta(): boolean {
    const snipes =
      state.currentBoard[CardLocation.BetaReserve * 3 + Offset.Snipes];
    return ((1 << Player.Alpha) & snipes) !== 0;
  }

  function isBetaSnipeCapturedByAlpha(): boolean {
    const snipes =
      state.currentBoard[CardLocation.AlphaReserve * 3 + Offset.Snipes];
    return ((1 << Player.Beta) & snipes) !== 0;
  }

  function getLegalAtomics(): Atomic[] {
    if (isEitherSnipeCaptured()) {
      return [];
    }

    const atomics: Atomic[] = [];

    // Only add full plies if there is no pending subply
    if (state.pendingAnimalStep === 0) {
      // Snipe steps
      {
        const activeSnipe = snipeOf(state.turn);
        const snipeLocation = getSnipeLocation(activeSnipe) as Row;
        const animals =
          state.currentBoard[snipeLocation * 3 + Offset.AlphaAnimals] |
          state.currentBoard[snipeLocation * 3 + Offset.BetaAnimals];
        const snipes = state.currentBoard[snipeLocation * 3 + Offset.Snipes];
        const enemySnipeFilter = 1 << opponentOf(state.turn);
        if ((snipes & enemySnipeFilter) | animals) {
          const forward = oneRowForward(snipeLocation, Player.Alpha);
          if (isRow(forward)) {
            atomics.push({
              plyType: PlyType.SnipeStep,
              destination: forward,
            });
          }

          const backward = oneRowBackward(snipeLocation, Player.Alpha);
          if (isRow(backward)) {
            atomics.push({
              plyType: PlyType.SnipeStep,
              destination: backward,
            });
          }
        }
      }

      // Drops
      {
        const reserve =
          state.turn === Player.Alpha
            ? CardLocation.AlphaReserve
            : CardLocation.BetaReserve;
        const friendlyAnimals =
          state.currentBoard[
            reserve * 3 +
              (state.turn === Player.Alpha
                ? Offset.AlphaAnimals
                : Offset.BetaAnimals)
          ];

        if (friendlyAnimals) {
          for (const cardType of allAnimalTypes) {
            if ((1 << cardType) & friendlyAnimals) {
              const desinations = canRetreat(cardType)
                ? legalRetreaterDrops[state.turn]
                : allRows;
              for (const row of desinations) {
                atomics.push({
                  plyType: PlyType.Drop,
                  dropped: cardType,
                  destination: row,
                });
              }
            }
          }
        }
      }
    }

    // Animal steps are always legal atomics, if there are any
    for (const row of allRows) {
      const friendlyAnimals =
        state.currentBoard[
          row * 3 +
            (state.turn === Player.Alpha
              ? Offset.AlphaAnimals
              : Offset.BetaAnimals)
        ];

      if (!friendlyAnimals) {
        continue;
      }

      const animals =
        state.currentBoard[row * 3 + Offset.AlphaAnimals] |
        state.currentBoard[row * 3 + Offset.BetaAnimals];
      const snipes = state.currentBoard[row * 3 + Offset.Snipes];
      const doesRowHaveAtLeastTwoCards = (bitCount(animals) >>> 1) | snipes;
      const enemySnipeFilter =
        state.turn === Player.Alpha ? 1 << Player.Beta : 1 << Player.Alpha;

      for (const cardType of allAnimalTypes) {
        if ((1 << cardType) & friendlyAnimals) {
          if (doesRowHaveAtLeastTwoCards) {
            const forward = oneRowForward(row, state.turn);
            if (isRow(forward)) {
              atomics.push({ moved: cardType, destination: forward });
            }

            if (canRetreat(cardType)) {
              const backward = oneRowBackward(row, state.turn);
              if (isRow(backward)) {
                atomics.push({ moved: cardType, destination: backward });
              }
            }
          } else {
            const forward = oneRowForward(row, state.turn);
            if (
              isRow(forward) &&
              snipes & enemySnipeFilter &&
              activatesTriplet(
                state.currentBoard[forward * 3 + Offset.AlphaAnimals] |
                  state.currentBoard[forward * 3 + Offset.BetaAnimals],
                cardType
              )
            ) {
              atomics.push({ moved: cardType, destination: forward });
            }

            if (canRetreat(cardType)) {
              const backward = oneRowBackward(row, state.turn);
              if (
                isRow(backward) &&
                snipes & enemySnipeFilter &&
                activatesTriplet(
                  state.currentBoard[backward * 3 + Offset.AlphaAnimals] |
                    state.currentBoard[backward * 3 + Offset.BetaAnimals],
                  cardType
                )
              ) {
                atomics.push({ moved: cardType, destination: backward });
              }
            }
          }
        }
      }
    }

    return atomics;
  }

  function isEitherSnipeCaptured() {
    return isAlphaSnipeCapturedByBeta() || isBetaSnipeCapturedByAlpha();
  }

  // https://stackoverflow.com/a/43122214/7215455
  function bitCount(n: number): number {
    n = n - ((n >> 1) & 0x55555555);
    n = (n & 0x33333333) + ((n >> 2) & 0x33333333);
    return (((n + (n >> 4)) & 0xf0f0f0f) * 0x1010101) >> 24;
  }

  function activatesTriplet(
    oldAnimals: number,
    newAnimal: AnimalType
  ): boolean {
    let rowElementCounts = cardProperties[newAnimal].elementCounts;

    for (const cardType of allAnimalTypes) {
      if ((1 << cardType) & oldAnimals) {
        rowElementCounts |= cardProperties[cardType].elementCounts;
      }
    }

    const [shift1, shift2] = cardProperties[newAnimal].tripletShifts;
    return (
      ((rowElementCounts >>> shift1) & 0b111) === 0b111 ||
      ((rowElementCounts >>> shift2) & 0b111) === 0b111
    );
  }

  function getTurn(): Player {
    return state.turn;
  }

  function getCardLocation(cardType: CardType): CardLocation {
    switch (cardType) {
      case CardType.AlphaSnipe:
      case CardType.BetaSnipe:
        return getSnipeLocation(cardType);
      default:
        return getAnimalLocation(cardType);
    }
  }

  function getSnipeLocation(cardType: SnipeType): CardLocation {
    const snipeFilter =
      cardType === CardType.AlphaSnipe ? 1 << Player.Alpha : 1 << Player.Beta;

    for (const location of allCardLocations) {
      const snipes = state.currentBoard[location * 3 + Offset.Snipes];
      if ((snipes & snipeFilter) !== 0) {
        return location;
      }
    }

    throw new Error(
      "Impossible: " + CardType[cardType] + " cannot be found in " + serialize()
    );
  }

  function getAnimalLocation(cardType: AnimalType): CardLocation {
    const animalFilter = 1 << cardType;

    for (const location of allCardLocations) {
      const animals =
        state.currentBoard[location * 3 + Offset.AlphaAnimals] |
        state.currentBoard[location * 3 + Offset.BetaAnimals];
      if ((animals & animalFilter) !== 0) {
        return location;
      }
    }

    throw new Error(
      "Impossible: " + CardType[cardType] + " cannot be found in " + serialize()
    );
  }

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
