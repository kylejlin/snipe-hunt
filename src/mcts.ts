import { Option, option } from "rusty-ts";
import { getAnalyzer } from "./analyzer";
import randInt from "./randInt";
import { Atomic, GameAnalyzer, GameState, Player } from "./types";

export interface MctsAnalyzer {
  performRollout(): void;
  getRoot(): Node;
  getBestAtomic(): Option<Atomic>;
  getChildWithBestAtomic(): Option<Node>;
}

export interface Node {
  edgeConnectingToParent: EdgeConnectingToParent | undefined;
  state: MinimalGameState;
  value: number;
  rollouts: number;
  children: Node[];
}

interface EdgeConnectingToParent {
  parent: Node;
  atomic: Atomic;
}

export interface MinimalGameState {
  currentBoard: Int32Array;
  turn: Player;
  pendingAnimalStep: number;
}

const EXPLORATION_CONSTANT = Math.sqrt(2);

export function getMctsAnalyzerIfStateIsNonTerminal(
  state: GameState
): Option<MctsAnalyzer> {
  const analyzer = getAnalyzer(state);
  if (analyzer.isGameOver()) {
    return option.none();
  } else {
    return option.some(getMctsAnalyzerForNonTerminalState(state, analyzer));
  }
}

function getMctsAnalyzerForNonTerminalState(
  state: GameState,
  analyzer: GameAnalyzer
): MctsAnalyzer {
  const root: Node = {
    edgeConnectingToParent: undefined,
    state,
    value: 0,
    rollouts: 0,
    children: [],
  };

  analyzer.setState(state);

  return { performRollout, getRoot, getBestAtomic, getChildWithBestAtomic };

  function performRollout(): void {
    let node = root;
    while (!isLeaf(node)) {
      const bestChild = selectBestChildAccordingToActivePlayer(node);
      node = bestChild;
    }
    const leaf = node;

    if (leaf.rollouts === 0 && leaf !== root) {
      rolloutIfNonTerminalThenBackPropagate(leaf);
    } else {
      analyzer.setState(leaf.state);
      analyzer.getWinner().match({
        some: (winner) => {
          const valueIncrease =
            winner === leaf.edgeConnectingToParent!.parent.state.turn ? 1 : 0;
          updateAndBackPropagate(leaf, valueIncrease);
        },
        none: () => {
          const children = getChildren(leaf);
          rolloutIfNonTerminalThenBackPropagate(children[0]);
          leaf.children = children;
        },
      });
    }
  }

  function isLeaf(node: Node): boolean {
    return node.children.length === 0;
  }

  function selectBestChildAccordingToActivePlayer(node: Node): Node {
    const { children } = node;

    let maxScore = getUcbScore(children[0], node.rollouts);
    let bestNode = children[0];

    for (let i = 1; i < children.length; i++) {
      const child = children[i];
      const score = getUcbScore(child, node.rollouts);
      if (score > maxScore) {
        maxScore = score;
        bestNode = child;
      }
    }

    return bestNode;
  }

  /**
   * Upper confidence bound (UCB1) based on
   * https://www.youtube.com/watch?v=UXW2yZndl7U
   */
  function getUcbScore(node: Node, parentRollouts: number) {
    if (node.rollouts === 0 || parentRollouts === 0) {
      return Infinity;
    }

    const meanValue = node.value / node.rollouts;
    return (
      meanValue +
      EXPLORATION_CONSTANT * Math.sqrt(Math.log(parentRollouts) / node.rollouts)
    );
  }

  function rolloutIfNonTerminalThenBackPropagate(node: Node): void {
    const perspective = node.edgeConnectingToParent!.parent.state.turn;

    analyzer.setState(node.state);
    const winner = analyzer.getWinner();

    const valueIncrease = winner.match({
      some: (winner) => {
        if (winner === perspective) {
          return 1;
        } else {
          return 0;
        }
      },

      none: () => {
        return rollout(node.state, perspective);
      },
    });

    updateAndBackPropagate(node, valueIncrease);
  }

  function setAnalyzerState(state: MinimalGameState): void {
    analyzer.setState(state as GameState);
  }

  function rollout(state: GameState, perspective: Player): 0 | 1 {
    analyzer.setState(state);

    let atomics = analyzer.getLegalAtomics();
    while (atomics.length > 0) {
      const selected = atomics[randInt(0, atomics.length)];
      analyzer.setState(analyzer.forcePerform(selected));
      atomics = analyzer.getLegalAtomics();
    }

    const winner = analyzer
      .getWinner()
      .expect("Impossible: No winner but no nextStates");
    if (winner === perspective) {
      return 1;
    } else {
      return 0;
    }
  }

  function updateAndBackPropagate(leaf: Node, valueIncrease: number): void {
    let node: Node | undefined = leaf;
    while (node !== undefined) {
      node.value += valueIncrease;
      node.rollouts += 1;
      node = node.edgeConnectingToParent?.parent;
    }
  }

  function getChildren(node: Node): Node[] {
    analyzer.setState(node.state);
    return analyzer.getLegalAtomics().map((atomic) => ({
      edgeConnectingToParent: {
        parent: node,
        atomic,
      },
      state: analyzer.forcePerform(atomic),
      value: 0,
      rollouts: 0,
      children: [],
    }));
  }

  function getRoot(): Node {
    return root;
  }

  function getBestAtomic(): Option<Atomic> {
    return getChildWithBestAtomic().map(
      (child) => child.edgeConnectingToParent!.atomic
    );
  }

  function getChildWithBestAtomic(): Option<Node> {
    let bestChild: Node | undefined = undefined;
    for (const child of root.children) {
      if (bestChild === undefined) {
        bestChild = child;
        continue;
      }

      if (child.rollouts > bestChild.rollouts) {
        bestChild = child;
      }
    }
    return option.fromVoidable(bestChild);
  }
}
