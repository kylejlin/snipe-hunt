import { Option, option, Result, result } from "rusty-ts";
import { Filter, Offset, PlyTag } from "./bitwiseUtils";
import { cardProperties } from "./cardMaps";
import {
  canRetreat,
  decodePly,
  isReserve,
  isRow,
  oneRowBackward,
  oneRowForward,
  opponentOf,
  snipeOf,
} from "./gameUtil";
import {
  allAnimalTypes,
  allCardLocations,
  allRows,
  AnimalStep,
  AnimalType,
  Atomic,
  Board,
  CardLocation,
  CardType,
  Drop,
  GameAnalyzer,
  GameState,
  IllegalGameStateUpdate,
  legalRetreaterDrops,
  Player,
  Ply,
  PlyType,
  Row,
  SnipeStep,
  SnipeType,
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
    tryUndoSubPly,
    tryPerform,
    serialize,
    toNodeKey,
    setState,
    getLegalAtomics,
    forcePerform,
  };

  function getInitialState(): GameState {
    return {
      stateVersion: state.stateVersion,
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
          const forward = oneRowForward(snipeLocation, state.turn);
          if (isRow(forward)) {
            atomics.push({
              plyType: PlyType.SnipeStep,
              destination: forward,
            });
          }

          const backward = oneRowBackward(snipeLocation, state.turn);
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

        if (bitCount(friendlyAnimals) > 1) {
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
        if (
          state.pendingAnimalStep &&
          ((state.pendingAnimalStep >>> 3) & Filter.LeastFiveBits) === cardType
        ) {
          continue;
        }

        if ((1 << cardType) & friendlyAnimals) {
          if (doesRowHaveAtLeastTwoCards) {
            const friendlySnipeFilter =
              state.turn === Player.Alpha
                ? 1 << Player.Alpha
                : 1 << Player.Beta;

            const forward = oneRowForward(row, state.turn);
            const forwardSnipes =
              state.currentBoard[forward * 3 + Offset.Snipes];
            if (
              isRow(forward) &&
              !(
                forwardSnipes & friendlySnipeFilter &&
                !(forwardSnipes & enemySnipeFilter) &&
                doesStepActivateTriplet(
                  state.currentBoard[forward * 3 + Offset.AlphaAnimals] |
                    state.currentBoard[forward * 3 + Offset.BetaAnimals],
                  cardType
                )
              )
            ) {
              atomics.push({ moved: cardType, destination: forward });
            }

            if (canRetreat(cardType)) {
              const backward = oneRowBackward(row, state.turn);
              const backwardSnipes =
                state.currentBoard[backward * 3 + Offset.Snipes];
              if (
                isRow(backward) &&
                !(
                  backwardSnipes & friendlySnipeFilter &&
                  !(backwardSnipes & enemySnipeFilter) &&
                  doesStepActivateTriplet(
                    state.currentBoard[backward * 3 + Offset.AlphaAnimals] |
                      state.currentBoard[backward * 3 + Offset.BetaAnimals],
                    cardType
                  )
                )
              ) {
                atomics.push({ moved: cardType, destination: backward });
              }
            }
          } else {
            const forward = oneRowForward(row, state.turn);
            if (
              isRow(forward) &&
              snipes & enemySnipeFilter &&
              doesStepActivateTriplet(
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
                doesStepActivateTriplet(
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

  function doesStepActivateTriplet(
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

  function tryUndoSubPly(): Result<
    { newState: GameState; undone: SnipeStep | Drop | AnimalStep },
    IllegalGameStateUpdate
  > {
    if (state.plies.length === 0 && state.pendingAnimalStep === 0) {
      return result.err(IllegalGameStateUpdate.NothingToUndo);
    }

    if (state.pendingAnimalStep !== 0) {
      const outOfSyncState = cloneState(state);
      outOfSyncState.pendingAnimalStep = 0;

      const undone: AnimalStep = {
        moved: (state.pendingAnimalStep >>> 3) & Filter.LeastFiveBits,
        destination: (state.pendingAnimalStep >>> 8) & Filter.LeastThreeBits,
      };
      return result.ok({
        newState: recalculateOutOfSyncGameState(outOfSyncState),
        undone,
      });
    }

    const outOfSyncState = cloneState(state);
    const encodedPly = outOfSyncState.plies.pop()!;
    const ply = decodePly(encodedPly);
    let undone: SnipeStep | Drop | AnimalStep;

    if (ply.plyType === PlyType.TwoAnimalSteps) {
      const encodedFirstStep =
        (ply.first.destination << 8) | (ply.first.moved << 3) | 0b001;
      outOfSyncState.pendingAnimalStep = encodedFirstStep;

      undone = ply.second;
    } else {
      undone = ply;
    }

    return result.ok({
      newState: recalculateOutOfSyncGameState(outOfSyncState),
      undone,
    });
  }

  function recalculateOutOfSyncGameState(
    originalMutState: GameState
  ): GameState {
    let mutState = originalMutState;
    const analyzer = getAnalyzer(mutState);

    mutState.currentBoard = new Int32Array(mutState.initialBoard);

    const plies = mutState.plies.map(decodePly);
    const encodedPendingAnimalStep = mutState.pendingAnimalStep;

    mutState.plies = [];
    mutState.pendingAnimalStep = 0;
    mutState.turn = Player.Beta;

    plies.forEach((ply) => {
      if (ply.plyType === PlyType.TwoAnimalSteps) {
        analyzer.setState(mutState);
        mutState = analyzer.forcePerform(ply.first);
        analyzer.setState(mutState);
        mutState = analyzer.forcePerform(ply.second);
      } else {
        analyzer.setState(mutState);
        mutState = analyzer.forcePerform(ply);
      }
    });

    if (encodedPendingAnimalStep) {
      analyzer.setState(mutState);
      const step: AnimalStep = {
        moved: (encodedPendingAnimalStep >>> 3) & Filter.LeastFiveBits,
        destination: (encodedPendingAnimalStep >>> 8) & Filter.LeastThreeBits,
      };
      mutState = analyzer.forcePerform(step);
    }

    return mutState;
  }

  function tryPerform(
    atomic: Atomic
  ): Result<GameState, IllegalGameStateUpdate> {
    return getReasonWhyAtomicIsIllegal(atomic).match({
      some: result.err,
      none: () => result.ok(forcePerform(atomic)),
    });
  }

  function getReasonWhyAtomicIsIllegal(
    atomic: Atomic
  ): Option<IllegalGameStateUpdate> {
    if (isEitherSnipeCaptured()) {
      return option.some(IllegalGameStateUpdate.SnipeAlreadyCaptured);
    }

    if ("plyType" in atomic) {
      switch (atomic.plyType) {
        case PlyType.SnipeStep: {
          if (state.pendingAnimalStep) {
            return option.some(IllegalGameStateUpdate.AlreadyMovedAnimal);
          }

          const moved = snipeOf(state.turn);
          if (!isInRange(moved, state.turn, atomic.destination)) {
            return option.some(
              IllegalGameStateUpdate.StepDestinationOutOfRange
            );
          }

          const location = getSnipeLocation(moved) as Row;
          const animals =
            state.currentBoard[location * 3 + Offset.AlphaAnimals] |
            state.currentBoard[location * 3 + Offset.BetaAnimals];
          const snipes = state.currentBoard[location * 3 + Offset.Snipes];
          const enemySnipeFilter =
            state.turn === Player.Alpha ? 1 << Player.Beta : 1 << Player.Alpha;
          if (!(animals | (snipes & enemySnipeFilter))) {
            return option.some(
              IllegalGameStateUpdate.CannotEmptyRowWithoutImmediatelyWinning
            );
          }

          return option.none();
        }

        case PlyType.Drop: {
          if (state.pendingAnimalStep) {
            return option.some(IllegalGameStateUpdate.AlreadyMovedAnimal);
          }

          const reserveAnimals =
            state.currentBoard[
              (state.turn === Player.Alpha
                ? CardLocation.AlphaReserve
                : CardLocation.BetaReserve) *
                3 +
                (state.turn === Player.Alpha
                  ? Offset.AlphaAnimals
                  : Offset.BetaAnimals)
            ];
          if (!((1 << atomic.dropped) & reserveAnimals)) {
            return option.some(
              IllegalGameStateUpdate.DroppedAnimalNotInReserve
            );
          }

          if (!(reserveAnimals & ~(1 << atomic.dropped))) {
            return option.some(IllegalGameStateUpdate.CannotEmptyReserve);
          }

          if (
            canRetreat(atomic.dropped) &&
            !legalRetreaterDrops[state.turn].includes(atomic.destination)
          ) {
            return option.some(
              IllegalGameStateUpdate.CannotDropRetreaterOnEnemysBackTwoRows
            );
          }

          return option.none();
        }
      }
    } else {
      const location = getAnimalLocation(atomic.moved);

      if (isReserve(location)) {
        return option.some(IllegalGameStateUpdate.MovedCardInReserve);
      }

      const friendlyAnimals =
        state.currentBoard[
          location * 3 +
            (state.turn === Player.Alpha
              ? Offset.AlphaAnimals
              : Offset.BetaAnimals)
        ];
      if (!((1 << atomic.moved) & friendlyAnimals)) {
        return option.some(IllegalGameStateUpdate.NotYourAnimal);
      }

      if (!isInRange(atomic.moved, state.turn, atomic.destination)) {
        return option.some(IllegalGameStateUpdate.StepDestinationOutOfRange);
      }

      if (
        state.pendingAnimalStep &&
        ((state.pendingAnimalStep >>> 3) & Filter.LeastFiveBits) ===
          atomic.moved
      ) {
        return option.some(IllegalGameStateUpdate.CannotMoveSameAnimalTwice);
      }

      const destAnimals =
        state.currentBoard[atomic.destination * 3 + Offset.AlphaAnimals] |
        state.currentBoard[atomic.destination * 3 + Offset.BetaAnimals];
      const activatesTriplet = doesStepActivateTriplet(
        destAnimals,
        atomic.moved
      );

      const destSnipes =
        state.currentBoard[atomic.destination * 3 + Offset.Snipes];
      const enemySnipeFilter =
        state.turn === Player.Alpha ? 1 << Player.Beta : 1 << Player.Alpha;
      const enemySnipeInDest = destSnipes & enemySnipeFilter;

      const sourceAnimals =
        state.currentBoard[location * 3 + Offset.AlphaAnimals] |
        state.currentBoard[location * 3 + Offset.BetaAnimals];
      const sourceSnipes = state.currentBoard[location * 3 + Offset.Snipes];
      const doesSourceRowHaveAnotherCard =
        (sourceAnimals & ~(1 << atomic.moved)) | sourceSnipes;

      const friendlySnipeFilter =
        state.turn === Player.Alpha ? 1 << Player.Alpha : 1 << Player.Beta;
      const friendlySnipeInDest = destSnipes & friendlySnipeFilter;

      if (doesSourceRowHaveAnotherCard) {
        if (activatesTriplet && friendlySnipeInDest && !enemySnipeInDest) {
          return option.some(
            IllegalGameStateUpdate.CannotCaptureOwnSnipeWithoutAlsoCapturingOpponents
          );
        } else {
          return option.none();
        }
      } else {
        if (activatesTriplet && enemySnipeInDest) {
          return option.none();
        } else {
          return option.some(
            IllegalGameStateUpdate.CannotEmptyRowWithoutImmediatelyWinning
          );
        }
      }
    }
  }

  function isInRange(
    movedType: CardType,
    movedAllegiance: Player,
    destination: Row
  ): boolean {
    const location = getCardLocation(movedType);

    if (isReserve(location)) {
      return false;
    }

    return (
      oneRowForward(location, movedAllegiance) === destination ||
      (canRetreat(movedType) &&
        oneRowBackward(location, movedAllegiance) === destination)
    );
  }

  function forcePerform(atomic: Atomic): GameState {
    if ("plyType" in atomic) {
      switch (atomic.plyType) {
        case PlyType.SnipeStep:
          return forcePerformSnipeStep(atomic);
        case PlyType.Drop:
          return forcePerformDrop(atomic);
      }
    } else {
      return forcePerformAnimalStep(atomic);
    }
  }

  function forcePerformSnipeStep(step: SnipeStep): GameState {
    const friendlySnipeLocation = getCardLocation(snipeOf(state.turn)) as Row;
    const friendlySnipeSet = 1 << state.turn;
    const removeFriendlySnipeFilter = ~friendlySnipeSet;

    const newState = cloneState(state);
    newState.currentBoard[
      friendlySnipeLocation * 3 + Offset.Snipes
    ] &= removeFriendlySnipeFilter;
    newState.currentBoard[
      step.destination * 3 + Offset.Snipes
    ] |= friendlySnipeSet;

    newState.plies.push((step.destination << 3) | PlyTag.SnipeStep);

    newState.turn = opponentOf(newState.turn);

    return newState;
  }

  function cloneState(src: GameState): GameState {
    return {
      stateVersion: src.stateVersion,
      initialBoard: new Int32Array(src.initialBoard),
      currentBoard: new Int32Array(src.currentBoard),
      turn: src.turn,
      plies: src.plies.slice(),
      pendingAnimalStep: src.pendingAnimalStep,
    };
  }

  function forcePerformDrop(drop: Drop): GameState {
    const reserve =
      state.turn === Player.Alpha
        ? CardLocation.AlphaReserve
        : CardLocation.BetaReserve;
    const friendlyOffset =
      state.turn === Player.Alpha ? Offset.AlphaAnimals : Offset.BetaAnimals;
    const droppedSet = 1 << drop.dropped;
    const removeDroppedFilter = ~droppedSet;

    const newState = cloneState(state);
    newState.currentBoard[reserve * 3 + friendlyOffset] &= removeDroppedFilter;
    newState.currentBoard[drop.destination * 3 + friendlyOffset] |= droppedSet;

    newState.plies.push(
      (drop.destination << 8) | (drop.dropped << 3) | PlyTag.Drop
    );

    newState.turn = opponentOf(newState.turn);

    return newState;
  }

  function forcePerformAnimalStep(step: AnimalStep): GameState {
    const start = getCardLocation(step.moved) as Row;
    const friendlyOffset =
      state.turn === Player.Alpha ? Offset.AlphaAnimals : Offset.BetaAnimals;

    const destAnimalsBeforeStep =
      state.currentBoard[step.destination * 3 + Offset.AlphaAnimals] |
      state.currentBoard[step.destination * 3 + Offset.BetaAnimals];

    const newState = cloneState(state);

    const movedSet = 1 << step.moved;
    const removeMovedFilter = ~movedSet;
    newState.currentBoard[start * 3 + friendlyOffset] &= removeMovedFilter;

    if (doesStepActivateTriplet(destAnimalsBeforeStep, step.moved)) {
      const friendlyReserve =
        state.turn === Player.Alpha
          ? CardLocation.AlphaReserve
          : CardLocation.BetaReserve;
      const destSnipesBeforeStep =
        state.currentBoard[step.destination * 3 + Offset.Snipes];
      const enemyOffset =
        state.turn === Player.Alpha ? Offset.BetaAnimals : Offset.AlphaAnimals;

      newState.currentBoard[
        friendlyReserve * 3 + friendlyOffset
      ] |= destAnimalsBeforeStep;
      newState.currentBoard[
        friendlyReserve * 3 + Offset.Snipes
      ] |= destSnipesBeforeStep;

      newState.currentBoard[step.destination * 3 + friendlyOffset] = movedSet;
      newState.currentBoard[step.destination * 3 + enemyOffset] = 0;
      newState.currentBoard[step.destination * 3 + Offset.Snipes] = 0;
    } else {
      newState.currentBoard[step.destination * 3 + friendlyOffset] |= movedSet;
    }

    if (newState.pendingAnimalStep) {
      const encodePlyWithIncorrectTag =
        (step.destination << 16) |
        (step.moved << 11) |
        newState.pendingAnimalStep;
      newState.plies.push(
        (encodePlyWithIncorrectTag & ~Filter.LeastThreeBits) |
          PlyTag.TwoAnimalSteps
      );
      newState.pendingAnimalStep = 0;
      newState.turn = opponentOf(newState.turn);
    } else {
      newState.pendingAnimalStep =
        (step.destination << 8) | (step.moved << 3) | 0b001;
    }

    return newState;
  }

  function serialize(): string {
    return JSON.stringify(state, (_key, value) => {
      if (value instanceof Int32Array) {
        return Array.from(value);
      } else {
        return value;
      }
    });
  }

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
}
