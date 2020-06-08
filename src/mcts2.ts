import { Option } from "rusty-ts";
import { MctsAnalyzer } from "./mcts";
import { GameState, Player } from "./types";

const ROOT_NODE_INDEX = 0;
const GUARANTEED_LOSS = -Infinity;
const GUARANTEED_WIN = Infinity;
const NO_WINNER = -1;

enum NodeOffsets {
  Rollouts = 20,
  ChildCount = 21,
  FirstChildIndex = 22,
}

type Booleanish = number;

export function getMctsAnalyzerIfStateIsNonTerminal(
  state: GameState,
  heapSize: number
): Option<MctsAnalyzer> {
  if (heapSize % Int32Array.BYTES_PER_ELEMENT !== 0) {
    throw new Error(
      "Heap size " +
        heapSize +
        " is not divisible by " +
        Int32Array.BYTES_PER_ELEMENT
    );
  }

  const heap = new Int32Array(heapSize / Int32Array.BYTES_PER_ELEMENT);

  return { performRollout, getRoot, getBestAtomic, getChildWithBestAtomic };

  function performRollout(): void {
    let nodeIndex = ROOT_NODE_INDEX;
    while (!isLeaf(nodeIndex)) {
      nodeIndex = selectBestChild(nodeIndex);
    }
    const leafIndex = nodeIndex;

    if (getRollouts(leafIndex) === 0 && leafIndex !== ROOT_NODE_INDEX) {
      rolloutIfNonTerminalThenBackPropagate(leafIndex);
    } else {
      const winner = getNodeStateWinner(leafIndex);
      if (winner === NO_WINNER) {
        addChildrenAndRolloutFirstChild(leafIndex);
      } else {
        updateAndBackPropagateRollout(leafIndex, winner);
        markAsTerminal(leafIndex, winner);
      }
    }
  }

  function isLeaf(nodeIndex: number): boolean {
    return heap[nodeIndex + NodeOffsets.ChildCount] === 0;
  }

  function selectBestChild(nodeIndex: number): number {
    const childCount = heap[nodeIndex + NodeOffsets.ChildCount];

    let bestIndex = -1;
    let bestScore = GUARANTEED_LOSS;
    for (let i = 0; i < childCount; i++) {
      const childIndex = heap[nodeIndex + NodeOffsets.FirstChildIndex + i];
      const childScore = getUcbScore(childIndex);

      if (childScore === GUARANTEED_WIN) {
        return childIndex;
      }

      if (childScore !== GUARANTEED_LOSS && childScore > bestScore) {
        bestIndex = childIndex;
        bestScore = childScore;
      }
    }
    return bestIndex;
  }

  function getRollouts(nodeIndex: number): number {
    return heap[nodeIndex + NodeOffsets.Rollouts];
  }

  function rolloutIfNonTerminalThenBackPropagate(nodeIndex: number): void {}

  function addChildrenAndRolloutFirstChild(nodeIndex: number): void {}

  function getUcbScore(nodeIndex: number): number {}

  function getNodeStateWinner(nodeIndex: number): Player | typeof NO_WINNER {}

  function updateAndBackPropagateRollout(
    leafIndex: number,
    winner: Player
  ): void {}

  function markAsTerminal(leafIndex: number, winner: Player): void {}
}
