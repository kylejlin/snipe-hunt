import { Option, option } from "rusty-ts";
import { getAnalyzer } from "./analyzer";
import { bitCount, Filter } from "./bitwiseUtils";
import { cardProperties } from "./cardMaps";
import { canRetreat, isRow, oneRowBackward, oneRowForward } from "./gameUtil";
import randInt from "./randInt";
import {
  allRows,
  AnimalStep,
  AnimalType,
  Atomic,
  CardLocation,
  Drop,
  GameState,
  legalRetreaterDrops,
  Player,
  PlyType,
  Row,
  SnipeStep,
} from "./types";

export const NODE_SIZE_IN_I32S = 22;

const ROOT_NODE_INDEX = 0;
const NO_WINNER = -1;
const STATE_SIZE_IN_I32S = 17;
const ALPHA_WINS = 0b11_1111_1110;
const BETA_WINS = 0b11_1111_1111;
const NO_MOVED_ANIMAL = 0b10_0000;

const EXPLORATION_CONSTANT = Math.sqrt(2);

export interface MctsAnalyzer {
  performRollout(): void;
  getRootSummary(): NodeSummary;
  getBestAtomic(): Atomic;
  getSummaryOfChildWithBestAtomic(): NodeSummary;
  getInternalData(): MctsAnalyzerInternalData;
  getAtomicsOnPathToNode(nodeIndex: number): Atomic[];
}

export interface NodeSummary {
  atomic: Option<Atomic>;
  value: number;
  rollouts: number;
}

export interface MctsAnalyzerInternalData {
  heapBuffer: ArrayBuffer;
  mallocIndex: number;
}

enum NodeOffsets {
  SnipeSetsTurnNumberAndMovedAnimal = 16,
  ParentIndex = 17,
  Atomic = 18,
  Value = 19,
  Rollouts = 20,
  ChildListStartIndex = 21,
}

type QuicklyPerformableAtomic =
  | QuicklyPerformableSnipeStep
  | QuicklyPerformableDrop
  | QuicklyPerformableAnimalStep;

enum QuicklyPerformableAtomicType {
  SnipeStep,
  Drop,
  AnimalStep,
}

interface QuicklyPerformableSnipeStep extends SnipeStep {
  qp_atomicType: QuicklyPerformableAtomicType.SnipeStep;
  qp_activePlayer: Player;
  qp_snipeLocation: Row;
}

interface QuicklyPerformableDrop extends Drop {
  qp_atomicType: QuicklyPerformableAtomicType.Drop;
  qp_activePlayer: Player;
}

interface QuicklyPerformableAnimalStep extends AnimalStep {
  qp_atomicType: QuicklyPerformableAtomicType.AnimalStep;
  qp_startIndex: number;
  qp_activePlayer: Player;
  qp_inactivePlayer: Player;
  qp_doesStepActivateTriplet: boolean;
}

export function getMctsAnalyzerIfStateIsNonTerminal(
  rootState: GameState,
  turnNumber: number,
  heapSizeInI32s: number
): Option<MctsAnalyzer> {
  const analyzer = getAnalyzer(rootState);
  if (analyzer.isGameOver()) {
    return option.none();
  } else {
    return option.some(
      getMctsAnalyzerForNonTerminalState(rootState, turnNumber, heapSizeInI32s)
    );
  }
}

function getMctsAnalyzerForNonTerminalState(
  rootState: GameState,
  turnNumber: number,
  heapSizeInI32s: number
): MctsAnalyzer {
  const heap = new Int32Array(heapSizeInI32s);
  let mallocIndex = 0;

  writeRootState();

  const uninitialized = getMctsAnalyzerFromHeapWithoutInitializing(
    heap,
    mallocIndex
  );

  uninitialized.performRollout();
  uninitialized.performRollout();

  return uninitialized;

  function writeRootState(): void {
    if (mallocIndex !== 0) {
      throw new Error("Cannot writeRootState() if heap is not empty.");
    }

    const { currentBoard, pendingAnimalStep } = rootState;

    heap[0] = currentBoard[0];
    heap[1] = currentBoard[1];

    heap[2] = currentBoard[3];
    heap[3] = currentBoard[4];

    heap[4] = currentBoard[6];
    heap[5] = currentBoard[7];

    heap[6] = currentBoard[9];
    heap[7] = currentBoard[10];

    heap[8] = currentBoard[12];
    heap[9] = currentBoard[13];

    heap[10] = currentBoard[15];
    heap[11] = currentBoard[16];

    heap[12] = currentBoard[18];
    heap[13] = currentBoard[19];

    heap[14] = currentBoard[21];
    heap[15] = currentBoard[22];

    heap[16] =
      ((pendingAnimalStep === 0
        ? NO_MOVED_ANIMAL
        : (pendingAnimalStep >>> 3) & Filter.LeastFiveBits) <<
        26) |
      (turnNumber << 16) |
      (currentBoard[23] << 14) |
      (currentBoard[20] << 12) |
      (currentBoard[17] << 10) |
      (currentBoard[14] << 8) |
      (currentBoard[11] << 6) |
      (currentBoard[8] << 4) |
      (currentBoard[5] << 2) |
      currentBoard[2];

    heap[NodeOffsets.ParentIndex] = -1;
    heap[NodeOffsets.Atomic] = -1;
    heap[NodeOffsets.Value] = 0;
    heap[NodeOffsets.Rollouts] = 0;
    heap[NodeOffsets.ChildListStartIndex] = -1;

    mallocIndex = 22;
  }
}

export function getMctsAnalyzerFromHeapWithoutInitializing(
  heap: Int32Array,
  initialMallocIndex: number
): MctsAnalyzer {
  const tempState = new Int32Array(STATE_SIZE_IN_I32S);
  let mallocIndex = initialMallocIndex;

  return {
    performRollout,
    getRootSummary,
    getBestAtomic,
    getSummaryOfChildWithBestAtomic,
    getInternalData,
    getAtomicsOnPathToNode,
  };

  function performRollout(): void {
    let nodeIndex = ROOT_NODE_INDEX;
    while (!isLeaf(nodeIndex)) {
      nodeIndex = selectBestChild(nodeIndex);
    }
    const leafIndex = nodeIndex;

    if (getRollouts(leafIndex) === 0 && leafIndex !== ROOT_NODE_INDEX) {
      rolloutIfNonTerminalThenBackPropagate(leafIndex);
    } else {
      const winner = getImmediateWinner(heap, leafIndex);
      if (~winner) {
        updateAndBackPropagateRollout(leafIndex, winner);
        markAsEffectivelyTerminal(leafIndex, winner);
      } else {
        addChildrenAndRolloutFirstChild(leafIndex);
      }
    }
  }

  function isLeaf(nodeIndex: number): boolean {
    const childListStartIndex =
      heap[nodeIndex + NodeOffsets.ChildListStartIndex];
    // If childListStartIndex is null (negative), then
    // heap[childListStartIndex] will be undefined,
    // and (undefined | 0) === 0.
    return (heap[childListStartIndex] | 0) === 0;
  }

  function selectBestChild(nodeIndex: number): number {
    const childListStartIndex =
      heap[nodeIndex + NodeOffsets.ChildListStartIndex];
    const childCount = heap[childListStartIndex];
    const nodeRollouts = getRollouts(nodeIndex);
    const turnNumberAndMovedAnimal =
      heap[nodeIndex + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] >>> 16;
    const activePlayer = turnNumberAndMovedAnimal & 0b1;
    const inactivePlayer = ~turnNumberAndMovedAnimal & 0b1;

    let bestIndex = getNthElementOfChildList(childListStartIndex, 0);
    let bestScore = getUcbScore(bestIndex, nodeRollouts);

    if (getEffectiveWinner(bestIndex) === activePlayer) {
      return bestIndex;
    }

    if (getEffectiveWinner(bestIndex) === inactivePlayer) {
      bestScore = -Infinity;
    }

    for (let i = 1; i < childCount; i++) {
      const childIndex = getNthElementOfChildList(childListStartIndex, i);
      const winner = getEffectiveWinner(childIndex);

      if (winner === activePlayer) {
        return childIndex;
      }

      if (winner === inactivePlayer) {
        continue;
      }

      const childScore = getUcbScore(childIndex, nodeRollouts);

      if (childScore > bestScore) {
        bestIndex = childIndex;
        bestScore = childScore;
      }
    }
    return bestIndex;
  }

  function getRollouts(nodeIndex: number): number {
    return heap[nodeIndex + NodeOffsets.Rollouts];
  }

  function getNthElementOfChildList(
    childListStartIndex: number,
    n: number
  ): number {
    return heap[childListStartIndex + 1 + n];
  }

  function rolloutIfNonTerminalThenBackPropagate(nodeIndex: number): void {
    const winner = getImmediateWinner(heap, nodeIndex);

    if (~winner) {
      markAsEffectivelyTerminal(nodeIndex, winner);
    }

    const rolledOutWinner = ~winner ? winner : rollout(nodeIndex);
    updateAndBackPropagateRollout(nodeIndex, rolledOutWinner);
  }

  function rollout(nodeIndex: number): Player {
    copyNodeStateIntoTempState(nodeIndex);
    let legalAtomics = getLegalAtomics(tempState);
    while (legalAtomics.length > 0) {
      const selected = legalAtomics[randInt(0, legalAtomics.length)];
      performAtomic(selected, tempState);
      legalAtomics = getLegalAtomics(tempState);
    }

    return getImmediateWinner(tempState, 0);
  }

  function copyNodeStateIntoTempState(nodeIndex: number): void {
    tempState.set(heap.subarray(nodeIndex, nodeIndex + STATE_SIZE_IN_I32S));
  }

  function getLegalAtomics(out: Int32Array): QuicklyPerformableAtomic[] {
    const etc = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];

    if ((etc & 0b10) | ((etc >>> 14) & 0b1)) {
      return [];
    }

    const atomics: QuicklyPerformableAtomic[] = [];

    const activePlayer: Player = (etc >>> 16) & 0b1;
    const inactivePlayer: Player = activePlayer ^ 0b1;
    const activePlayerLegalRetreaterDrops = legalRetreaterDrops[activePlayer];

    // Only add full plies if there is no pending subply
    if (getOptMovedAnimal(out) === NO_MOVED_ANIMAL) {
      // Snipe steps
      {
        if (snipeCannotBeFound(etc, activePlayer)) {
          console.log("etc", etc);
          debugger;
        }

        const activeSnipeLocation = getActiveSnipeLocation(etc, activePlayer);
        if (isRow(activeSnipeLocation)) {
          const enemySnipeSet =
            (etc >>> (activeSnipeLocation << 1)) & (1 << inactivePlayer);
          const animalSet =
            out[(activeSnipeLocation << 1) | activePlayer] |
            out[(activeSnipeLocation << 1) | inactivePlayer];
          if (enemySnipeSet | animalSet) {
            const forward = oneRowForward(activeSnipeLocation, activePlayer);
            if (isRow(forward)) {
              atomics.push({
                plyType: PlyType.SnipeStep,
                destination: forward,

                qp_atomicType: QuicklyPerformableAtomicType.SnipeStep,
                qp_activePlayer: activePlayer,
                qp_snipeLocation: activeSnipeLocation,
              });
            }

            const backward = oneRowBackward(activeSnipeLocation, activePlayer);
            if (isRow(backward)) {
              atomics.push({
                plyType: PlyType.SnipeStep,
                destination: backward,

                qp_atomicType: QuicklyPerformableAtomicType.SnipeStep,
                qp_activePlayer: activePlayer,
                qp_snipeLocation: activeSnipeLocation,
              });
            }
          }
        }
      }

      // Drops
      {
        const reserveAnimalSet = getReserveAnimalSet(activePlayer, out);

        if (bitCount(reserveAnimalSet) > 1) {
          for (let animalType = 0; animalType <= 31; animalType++) {
            if ((1 << animalType) & reserveAnimalSet) {
              const destinations = canRetreat(animalType)
                ? activePlayerLegalRetreaterDrops
                : allRows;
              const destinationsLen = destinations.length;
              for (let i = 0; i < destinationsLen; i++) {
                atomics.push({
                  plyType: PlyType.Drop,
                  dropped: animalType,
                  destination: destinations[i],

                  qp_atomicType: QuicklyPerformableAtomicType.Drop,
                  qp_activePlayer: activePlayer,
                });
              }
            }
          }
        }
      }
    }

    // Animal steps can be performed regardless of the previous action
    for (let row: Row = 1; row <= 6; row++) {
      const startIndex = (row << 1) | activePlayer;
      const friendlyAnimalSet = out[startIndex];

      if (friendlyAnimalSet === 0) {
        continue;
      }

      const animalSet = friendlyAnimalSet | out[(row << 1) + inactivePlayer];
      const snipes = (etc >>> (row << 1)) & Filter.LeastTwoBits;
      const doesRowHaveAtLeastTwoCards = snipes || bitCount(animalSet) > 1;
      const previouslyMovedAnimal = etc >>> 26;

      const forward = oneRowForward(row, activePlayer);
      const isForwardRow = forward !== 0 && forward !== 7;
      const isForwardNotRow: 0 | 2 = (((!isForwardRow as unknown) as 0 | 1) <<
        1) as 0 | 2;
      const forwardSnipesAndExtraneousGreaterBits = etc >>> (forward << 1);
      const isFriendlySnipeInForwardRow =
        (forwardSnipesAndExtraneousGreaterBits & (1 << activePlayer)) >>>
        isForwardNotRow;
      const isEnemySnipeInForwardRow =
        (forwardSnipesAndExtraneousGreaterBits & (1 << inactivePlayer)) >>>
        isForwardNotRow;

      const backward = oneRowBackward(row, activePlayer);
      const isBackwardRow = backward !== 0 && backward !== 7;
      const isBackwardNotRow: 0 | 2 = (((!isBackwardRow as unknown) as 0 | 1) <<
        1) as 0 | 2;
      const backwardSnipesAndExtraneousGreaterBits = etc >>> (backward << 1);
      const isFriendlySnipeInBackwardRow =
        (backwardSnipesAndExtraneousGreaterBits & (1 << activePlayer)) >>>
        isBackwardNotRow;
      const isEnemySnipeInBackwardRow =
        (backwardSnipesAndExtraneousGreaterBits & (1 << inactivePlayer)) >>>
        isBackwardNotRow;

      for (let animalType = 0; animalType <= 31; animalType++) {
        if (previouslyMovedAnimal === animalType) {
          continue;
        }

        if ((1 << animalType) & friendlyAnimalSet) {
          const doesForwardStepActivateTriplet = doesStepActivateTriplet(
            forward,
            animalType,
            out
          );
          const canThisAnimalRetreat = canRetreat(animalType);
          const canThisAnimalActivateTripletByRetreating =
            canThisAnimalRetreat &&
            doesStepActivateTriplet(backward, animalType, out);

          if (doesRowHaveAtLeastTwoCards) {
            if (
              isForwardRow &&
              !(
                isFriendlySnipeInForwardRow &&
                !isEnemySnipeInForwardRow &&
                doesForwardStepActivateTriplet
              )
            ) {
              atomics.push({
                moved: animalType,
                destination: forward as Row,

                qp_atomicType: QuicklyPerformableAtomicType.AnimalStep,
                qp_startIndex: startIndex,
                qp_activePlayer: activePlayer,
                qp_inactivePlayer: inactivePlayer,
                qp_doesStepActivateTriplet: doesForwardStepActivateTriplet,
              });
            }

            if (canThisAnimalRetreat) {
              if (
                isBackwardRow &&
                !(
                  isFriendlySnipeInBackwardRow &&
                  !isEnemySnipeInBackwardRow &&
                  canThisAnimalActivateTripletByRetreating
                )
              ) {
                atomics.push({
                  moved: animalType,
                  destination: backward as Row,

                  qp_atomicType: QuicklyPerformableAtomicType.AnimalStep,
                  qp_startIndex: startIndex,
                  qp_activePlayer: activePlayer,
                  qp_inactivePlayer: inactivePlayer,
                  qp_doesStepActivateTriplet: canThisAnimalActivateTripletByRetreating,
                });
              }
            }
          } else {
            if (
              isForwardRow &&
              isEnemySnipeInForwardRow &&
              doesForwardStepActivateTriplet
            ) {
              atomics.push({
                moved: animalType,
                destination: forward as Row,

                qp_atomicType: QuicklyPerformableAtomicType.AnimalStep,
                qp_startIndex: startIndex,
                qp_activePlayer: activePlayer,
                qp_inactivePlayer: inactivePlayer,
                qp_doesStepActivateTriplet: doesForwardStepActivateTriplet,
              });
            }

            if (
              canThisAnimalActivateTripletByRetreating &&
              isBackwardRow &&
              isEnemySnipeInBackwardRow
            ) {
              atomics.push({
                moved: animalType,
                destination: backward as Row,

                qp_atomicType: QuicklyPerformableAtomicType.AnimalStep,
                qp_startIndex: startIndex,
                qp_activePlayer: activePlayer,
                qp_inactivePlayer: inactivePlayer,
                qp_doesStepActivateTriplet: canThisAnimalActivateTripletByRetreating,
              });
            }
          }
        }
      }
    }

    return atomics;
  }

  function getOptMovedAnimal(out: Int32Array): number {
    return out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] >>> 26;
  }

  function getActiveSnipeLocation(
    etc: number,
    activePlayer: Player
  ): CardLocation {
    const activeSnipe = 1 << activePlayer;
    for (let i = 0; i <= 14; i += 2) {
      if ((etc >>> i) & activeSnipe) {
        return i >>> 1;
      }
    }

    throw new Error(
      "Impossible: " + Player[activePlayer] + " snipe cannot be found."
    );
  }

  function snipeCannotBeFound(etc: number, activePlayer: Player): boolean {
    const activeSnipe = 1 << activePlayer;
    for (let i = 0; i <= 14; i += 2) {
      if ((etc >>> i) & activeSnipe) {
        return false;
      }
    }

    return true;
  }

  function getReserveAnimalSet(player: Player, out: Int32Array): number {
    return out[getReserveIndex(player)];
  }

  function getReserveIndex(player: Player): 0 | 15 {
    if (player === Player.Alpha) {
      return 0;
    } else {
      return 15;
    }
  }

  function doesStepActivateTriplet(
    destination: CardLocation,
    newAnimal: AnimalType,
    out: Int32Array
  ): boolean {
    const oldAnimals = out[destination << 1] | out[(destination << 1) + 1];

    let rowElementCounts = cardProperties[newAnimal].elementCounts;

    for (let animalType: AnimalType = 0; animalType <= 31; animalType++) {
      if ((1 << animalType) & oldAnimals) {
        rowElementCounts |= cardProperties[animalType].elementCounts;
      }
    }

    const [shift1, shift2] = cardProperties[newAnimal].tripletShifts;
    return (
      ((rowElementCounts >>> shift1) & 0b111) === 0b111 ||
      ((rowElementCounts >>> shift2) & 0b111) === 0b111
    );
  }

  function performAtomic(
    atomic: QuicklyPerformableAtomic,
    out: Int32Array
  ): void {
    if (atomic.qp_atomicType === QuicklyPerformableAtomicType.SnipeStep) {
      return performSnipeStep(atomic, out);
    } else if (atomic.qp_atomicType === QuicklyPerformableAtomicType.Drop) {
      return performDrop(atomic, out);
    } else {
      return performAnimalStep(atomic, out);
    }
  }

  function performSnipeStep(
    atomic: QuicklyPerformableSnipeStep,
    out: Int32Array
  ): void {
    const debug_before = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];

    const snipe = 1 << atomic.qp_activePlayer;
    out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] &= ~(
      snipe <<
      (atomic.qp_snipeLocation << 1)
    );
    out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] |=
      snipe << (atomic.destination << 1);

    const debug_after = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];

    if (
      snipeCannotBeFound(debug_after, 0) ||
      snipeCannotBeFound(debug_after, 1)
    ) {
      console.log(
        "before",
        debug_before,
        "after",
        debug_after,
        "was already broken",
        snipeCannotBeFound(debug_before, 0) ||
          snipeCannotBeFound(debug_before, 1),
        "atomic",
        atomic
      );
    }

    const etc = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];
    const turnNumber = (etc >>> 16) & Filter.LeastTenBits;
    out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] =
      (etc & 0b1111_1100_0000_0000_1111_1111_1111_1111) |
      ((turnNumber + 1) << 16);
  }

  function performDrop(atomic: QuicklyPerformableDrop, out: Int32Array): void {
    const reserveIndex = getReserveIndex(atomic.qp_activePlayer);
    const droppedSet = 1 << atomic.dropped;

    out[reserveIndex] &= ~droppedSet;
    out[(atomic.destination << 1) + atomic.qp_activePlayer] |= droppedSet;

    const etc = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];
    const turnNumber = (etc >>> 16) & Filter.LeastTenBits;
    out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] =
      (etc & 0b1111_1100_0000_0000_1111_1111_1111_1111) |
      ((turnNumber + 1) << 16);
  }

  function performAnimalStep(
    atomic: QuicklyPerformableAnimalStep,
    out: Int32Array
  ): void {
    const startIndex = atomic.qp_startIndex;
    const movedSet = 1 << atomic.moved;

    out[startIndex] &= ~movedSet;

    const destAlphaIndex = atomic.destination << 1;

    if (atomic.qp_doesStepActivateTriplet) {
      out[getReserveIndex(atomic.qp_activePlayer)] |=
        out[destAlphaIndex] | out[destAlphaIndex | 0b1];
      out[destAlphaIndex | atomic.qp_activePlayer] = movedSet;
      out[destAlphaIndex | atomic.qp_inactivePlayer] = 0;

      const etc = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];
      const capturedSnipesSet = (etc >>> destAlphaIndex) & Filter.LeastTwoBits;
      out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] =
        (etc & ~(0b11 << destAlphaIndex)) |
        (capturedSnipesSet << (atomic.qp_activePlayer ? 14 : 0));
    } else {
      out[destAlphaIndex | atomic.qp_activePlayer] |= movedSet;
    }

    const etc = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];
    if (etc >>> 31) {
      // If moved animal does not exist, set moved animal to atomic.moved.
      out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] =
        (etc & 0b0000_0011_1111_1111_1111_1111_1111_1111) |
        (atomic.moved << 26);
    } else {
      // If moved animal exists, set moved animal to null and increment the turn number.
      out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] =
        (((etc & 0b0000_0011_1111_1111_1111_1111_1111_1111) |
          0b1000_0000_0000_0000_0000_0000_0000_0000) &
          0b1111_1100_0000_0000_1111_1111_1111_1111) |
        ((((etc >>> 16) & Filter.LeastTenBits) + 1) << 16);
    }
  }

  function getImmediateWinner(
    state: Int32Array,
    offset: number
  ): Player | typeof NO_WINNER {
    const etc = state[offset + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];
    if (etc & 0b10) {
      return Player.Alpha;
    } else if ((etc >>> 14) & 0b01) {
      return Player.Beta;
    } else if (
      !hasAtLeastOneLegalAtomic(
        state.subarray(offset, offset + STATE_SIZE_IN_I32S)
      )
    ) {
      return ~(etc >>> 16) & 0b1;
    } else {
      return NO_WINNER;
    }
  }

  function getEffectiveWinner(nodeIndex: number): Player | typeof NO_WINNER {
    const turnNumber =
      (heap[nodeIndex + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] >>> 16) &
      Filter.LeastTenBits;

    if (turnNumber === ALPHA_WINS) {
      return Player.Alpha;
    } else if (turnNumber === BETA_WINS) {
      return Player.Beta;
    } else {
      return NO_WINNER;
    }
  }

  function hasAtLeastOneLegalAtomic(out: Int32Array): boolean {
    const etc = out[NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal];

    if ((etc & 0b10) | ((etc >>> 14) & 0b1)) {
      return false;
    }

    const activePlayer: Player = (etc >>> 16) & 0b1;
    const inactivePlayer: Player = activePlayer ^ 0b1;
    const activePlayerLegalRetreaterDrops = legalRetreaterDrops[activePlayer];

    // Only add full plies if there is no pending subply
    if (getOptMovedAnimal(out) === NO_MOVED_ANIMAL) {
      // Snipe steps
      {
        if (snipeCannotBeFound(etc, activePlayer)) {
          console.log("etc", etc);
          debugger;
        }

        const activeSnipeLocation = getActiveSnipeLocation(etc, activePlayer);
        if (isRow(activeSnipeLocation)) {
          const enemySnipeSet =
            (etc >>> (activeSnipeLocation << 1)) & (1 << inactivePlayer);
          const animalSet =
            out[(activeSnipeLocation << 1) | activePlayer] |
            out[(activeSnipeLocation << 1) | inactivePlayer];
          if (enemySnipeSet | animalSet) {
            const forward = oneRowForward(activeSnipeLocation, activePlayer);
            if (isRow(forward)) {
              return true;
            }

            const backward = oneRowBackward(activeSnipeLocation, activePlayer);
            if (isRow(backward)) {
              return true;
            }
          }
        }
      }

      // Drops
      {
        const reserveAnimalSet = getReserveAnimalSet(activePlayer, out);

        if (bitCount(reserveAnimalSet) > 1) {
          for (let animalType = 0; animalType <= 31; animalType++) {
            if ((1 << animalType) & reserveAnimalSet) {
              const destinations = canRetreat(animalType)
                ? activePlayerLegalRetreaterDrops
                : allRows;
              const destinationsLen = destinations.length;
              for (let i = 0; i < destinationsLen; i++) {
                return true;
              }
            }
          }
        }
      }
    }

    // Animal steps can be performed regardless of the previous action
    for (let row: Row = 1; row <= 6; row++) {
      const startIndex = (row << 1) | activePlayer;
      const friendlyAnimalSet = out[startIndex];

      if (friendlyAnimalSet === 0) {
        continue;
      }

      const animalSet = friendlyAnimalSet | out[(row << 1) + inactivePlayer];
      const snipes = (etc >>> (row << 1)) & Filter.LeastTwoBits;
      const doesRowHaveAtLeastTwoCards = snipes || bitCount(animalSet) > 1;
      const previouslyMovedAnimal = etc >>> 26;

      const forward = oneRowForward(row, activePlayer);
      const isForwardRow = forward !== 0 && forward !== 7;
      const isForwardNotRow: 0 | 2 = (((!isForwardRow as unknown) as 0 | 1) <<
        1) as 0 | 2;
      const forwardSnipesAndExtraneousGreaterBits = etc >>> (forward << 1);
      const isFriendlySnipeInForwardRow =
        (forwardSnipesAndExtraneousGreaterBits & (1 << activePlayer)) >>>
        isForwardNotRow;
      const isEnemySnipeInForwardRow =
        (forwardSnipesAndExtraneousGreaterBits & (1 << inactivePlayer)) >>>
        isForwardNotRow;

      const backward = oneRowBackward(row, activePlayer);
      const isBackwardRow = backward !== 0 && backward !== 7;
      const isBackwardNotRow: 0 | 2 = (((!isBackwardRow as unknown) as 0 | 1) <<
        1) as 0 | 2;
      const backwardSnipesAndExtraneousGreaterBits = etc >>> (backward << 1);
      const isFriendlySnipeInBackwardRow =
        (backwardSnipesAndExtraneousGreaterBits & (1 << activePlayer)) >>>
        isBackwardNotRow;
      const isEnemySnipeInBackwardRow =
        (backwardSnipesAndExtraneousGreaterBits & (1 << inactivePlayer)) >>>
        isBackwardNotRow;

      for (let animalType = 0; animalType <= 31; animalType++) {
        if (previouslyMovedAnimal === animalType) {
          continue;
        }

        if ((1 << animalType) & friendlyAnimalSet) {
          const doesForwardStepActivateTriplet = doesStepActivateTriplet(
            forward,
            animalType,
            out
          );
          const canThisAnimalRetreat = canRetreat(animalType);
          const canThisAnimalActivateTripletByRetreating =
            canThisAnimalRetreat &&
            doesStepActivateTriplet(backward, animalType, out);

          if (doesRowHaveAtLeastTwoCards) {
            if (
              isForwardRow &&
              !(
                isFriendlySnipeInForwardRow &&
                !isEnemySnipeInForwardRow &&
                doesForwardStepActivateTriplet
              )
            ) {
              return true;
            }

            if (canThisAnimalRetreat) {
              if (
                isBackwardRow &&
                !(
                  isFriendlySnipeInBackwardRow &&
                  !isEnemySnipeInBackwardRow &&
                  canThisAnimalActivateTripletByRetreating
                )
              ) {
                return true;
              }
            }
          } else {
            if (
              isForwardRow &&
              isEnemySnipeInForwardRow &&
              doesForwardStepActivateTriplet
            ) {
              return true;
            }

            if (
              canThisAnimalActivateTripletByRetreating &&
              isBackwardRow &&
              isEnemySnipeInBackwardRow
            ) {
              return true;
            }
          }
        }
      }
    }

    return false;
  }

  function addChildrenAndRolloutFirstChild(nodeIndex: number): void {
    const childIndexes: number[] = [];

    const atomics = getLegalAtomics(
      heap.subarray(nodeIndex, nodeIndex + STATE_SIZE_IN_I32S)
    );
    const atomicsLen = atomics.length;
    for (let i = 0; i < atomicsLen; i++) {
      const atomic = atomics[i];

      const childIndex = malloc(NODE_SIZE_IN_I32S);
      childIndexes.push(childIndex);
      createChildOf(nodeIndex, atomic, childIndex);
    }

    const childIndexesLen = childIndexes.length;
    const childListStartIndex = malloc(1 + childIndexesLen);

    heap[nodeIndex + NodeOffsets.ChildListStartIndex] = childListStartIndex;

    heap[childListStartIndex] = childIndexesLen;

    for (let i = 0; i < childIndexesLen; i++) {
      heap[childListStartIndex + 1 + i] = childIndexes[i];
    }

    if (childIndexesLen) {
      rolloutIfNonTerminalThenBackPropagate(childIndexes[0]);
    }
  }

  function malloc(space: number): number {
    if (mallocIndex > heap.length - space) {
      throw new Error("Monte Carlo Tree heap has ran out of space.");
    }

    const address = mallocIndex;
    mallocIndex += space;
    return address;
  }

  function createChildOf(
    nodeIndex: number,
    atomic: QuicklyPerformableAtomic,
    destIndex: number
  ): void {
    heap.set(heap.subarray(nodeIndex, nodeIndex + 17), destIndex);

    heap[destIndex + NodeOffsets.ParentIndex] = nodeIndex;
    heap[destIndex + NodeOffsets.Atomic] = encodeAtomic(atomic);
    heap[destIndex + NodeOffsets.Value] = 0;
    heap[destIndex + NodeOffsets.Rollouts] = 0;
    heap[destIndex + NodeOffsets.ChildListStartIndex] = -1;

    performAtomic(
      atomic,
      heap.subarray(destIndex, destIndex + STATE_SIZE_IN_I32S)
    );
  }

  function encodeAtomic(atomic: QuicklyPerformableAtomic): number {
    if (atomic.qp_atomicType === QuicklyPerformableAtomicType.SnipeStep) {
      return (atomic.destination << 3) | 0b001;
    } else if (atomic.qp_atomicType === QuicklyPerformableAtomicType.Drop) {
      return (atomic.destination << 8) | (atomic.dropped << 3) | 0b010;
    } else {
      return (atomic.destination << 8) | (atomic.moved << 3) | 0b011;
    }
  }

  function decodeAtomic(encoded: number): Atomic {
    const tag = encoded & Filter.LeastThreeBits;
    if (tag === 0b001) {
      return {
        plyType: PlyType.SnipeStep,
        destination: encoded >>> 3,
      };
    } else if (tag === 0b010) {
      return {
        plyType: PlyType.Drop,
        dropped: (encoded >>> 3) & Filter.LeastFiveBits,
        destination: encoded >>> 8,
      };
    } else {
      return {
        moved: (encoded >>> 3) & Filter.LeastFiveBits,
        destination: encoded >>> 8,
      };
    }
  }

  function getUcbScore(nodeIndex: number, parentRollouts: number): number {
    const nodeRollouts = getRollouts(nodeIndex);
    if (nodeRollouts === 0) {
      return Infinity;
    }

    const meanValue = getValue(nodeIndex) / nodeRollouts;
    return (
      meanValue +
      EXPLORATION_CONSTANT * Math.sqrt(Math.log(parentRollouts) / nodeRollouts)
    );
  }

  function getValue(nodeIndex: number): number {
    return heap[nodeIndex + NodeOffsets.Value];
  }

  function updateAndBackPropagateRollout(
    leafIndex: number,
    winner: Player
  ): void {
    let nodeIndex = leafIndex;
    let parentIndex = heap[nodeIndex + NodeOffsets.ParentIndex];

    while (~parentIndex) {
      heap[nodeIndex + NodeOffsets.Value] += ((winner ===
        getActivePlayerOfNonTerminalState(parentIndex)) as unknown) as 1 | 0;
      heap[nodeIndex + NodeOffsets.Rollouts] += 1;

      nodeIndex = parentIndex;
      parentIndex = heap[nodeIndex + NodeOffsets.ParentIndex];
    }

    heap[nodeIndex + NodeOffsets.Value] += ((winner ===
      getActivePlayerOfNonTerminalState(nodeIndex)) as unknown) as 1 | 0;
    heap[nodeIndex + NodeOffsets.Rollouts] += 1;
  }

  function getActivePlayerOfNonTerminalState(nodeIndex: number): Player {
    return (
      (heap[nodeIndex + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] >>> 16) &
      0b1
    );
  }

  function markAsEffectivelyTerminal(leafIndex: number, winner: Player): void {
    const winnerBits = (0b11_1111_1110 | winner) << 16;

    let nodeIndex = leafIndex;
    let parentIndex = getParentIndex(nodeIndex);

    while (
      ~parentIndex &&
      ((heap[parentIndex + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] >>>
        16) &
        0b1) ===
        winner
    ) {
      const etcIndex =
        nodeIndex + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal;
      const etc = heap[etcIndex];
      heap[etcIndex] =
        (etc & 0b1111_1100_0000_0000_1111_1111_1111_1111) | winnerBits;

      nodeIndex = parentIndex;
      parentIndex = getParentIndex(nodeIndex);
    }

    const etcIndex = nodeIndex + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal;
    const etc = heap[etcIndex];
    heap[etcIndex] =
      (etc & 0b1111_1100_0000_0000_1111_1111_1111_1111) | winnerBits;
  }

  function getParentIndex(nodeIndex: number): number {
    return heap[nodeIndex + NodeOffsets.ParentIndex];
  }

  function getRootSummary(): NodeSummary {
    return {
      atomic: option.none(),
      value: heap[ROOT_NODE_INDEX + NodeOffsets.Value],
      rollouts: heap[ROOT_NODE_INDEX + NodeOffsets.Rollouts],
    };
  }

  function getBestAtomic(): Atomic {
    return getSummaryOfChildWithBestAtomic().atomic.expect(
      "Impossible: Child node does not have atomic."
    );
  }

  function getSummaryOfChildWithBestAtomic(): NodeSummary {
    const bestChildIndex = getIndexOfChildWithBestAtomic();

    return {
      atomic: option.some(
        decodeAtomic(heap[bestChildIndex + NodeOffsets.Atomic])
      ),
      value: heap[bestChildIndex + NodeOffsets.Value],
      rollouts: heap[bestChildIndex + NodeOffsets.Rollouts],
    };
  }

  function getIndexOfChildWithBestAtomic(): number {
    const rootActivePlayer =
      (heap[ROOT_NODE_INDEX + NodeOffsets.SnipeSetsTurnNumberAndMovedAnimal] >>>
        16) &
      0b1;
    const rootInactivePlayer = ~rootActivePlayer & 0b1;

    const rootChildListStartIndex =
      heap[ROOT_NODE_INDEX + NodeOffsets.ChildListStartIndex];
    const rootChildListLen = heap[rootChildListStartIndex];

    let bestChildIndex = -1;
    for (let i = 0; i < rootChildListLen; i++) {
      const childIndex = heap[rootChildListStartIndex + 1 + i];

      if (bestChildIndex === -1) {
        bestChildIndex = childIndex;
        continue;
      }

      const winner = getEffectiveWinner(childIndex);

      if (winner === rootActivePlayer) {
        return childIndex;
      }

      if (winner === rootInactivePlayer) {
        continue;
      }

      if (
        heap[childIndex + NodeOffsets.Rollouts] >
          heap[bestChildIndex + NodeOffsets.Rollouts] ||
        getEffectiveWinner(bestChildIndex) === rootInactivePlayer
      ) {
        bestChildIndex = childIndex;
      }
    }

    if (bestChildIndex === -1) {
      throw new Error("Impossible: Root node has no children.");
    }

    return bestChildIndex;
  }

  function getInternalData(): MctsAnalyzerInternalData {
    return { heapBuffer: heap.buffer, mallocIndex };
  }

  function getAtomicsOnPathToNode(leafIndex: number): Atomic[] {
    const atomics: Atomic[] = [];

    let nodeIndex = leafIndex;
    let parentIndex = getParentIndex(nodeIndex);
    while (~parentIndex) {
      atomics.push(decodeAtomic(heap[nodeIndex + NodeOffsets.Atomic]));

      nodeIndex = parentIndex;
      parentIndex = getParentIndex(nodeIndex);
    }

    return atomics.reverse();
  }
}
