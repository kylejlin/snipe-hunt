import { Option, option } from "rusty-ts";
import { getMinimalGameStateAnalyzer } from "./minimalAnalyzer";
import randInt from "./randInt";
import {
  Atomic,
  GameState,
  MinimalGameState,
  Player,
  MinimalGameAnalyzer,
} from "./types";

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

const EXPLORATION_CONSTANT = Math.sqrt(2);

export function getMctsAnalyzerIfStateIsNonTerminal(
  state: GameState
): Option<MctsAnalyzer> {
  const analyzer = getMinimalGameStateAnalyzer(state);
  if (analyzer.isGameOver()) {
    return option.none();
  } else {
    return option.some(getMctsAnalyzerForNonTerminalState(state, analyzer));
  }
}

function getMctsAnalyzerForNonTerminalState(
  state: GameState,
  analyzer: MinimalGameAnalyzer
): MctsAnalyzer {
  const root: Node = {
    edgeConnectingToParent: undefined,
    state,
    value: 0,
    rollouts: 0,
    children: [],
  };
  return getMctsAnalyzerForNonTerminalStateFromRoot(root, analyzer);
}

export function getMctsAnalyzerForNonTerminalStateFromRoot(
  root: Node,
  analyzer: MinimalGameAnalyzer
): MctsAnalyzer {
  analyzer.setState(root.state);

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
          updateAndBackPropagate(leaf, winner);
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
    analyzer.setState(node.state);
    const optWinner = analyzer.getWinner();

    const winner = optWinner.unwrapOrElse(() => rollout(node.state));

    updateAndBackPropagate(node, winner);
  }

  function rollout(state: MinimalGameState): Player {
    analyzer.setState(state);

    let atomics = analyzer.getLegalAtomics();
    while (atomics.length > 0) {
      const selected = atomics[randInt(0, atomics.length)];
      analyzer.setState(analyzer.forcePerform(selected));
      atomics = analyzer.getLegalAtomics();
    }

    return analyzer
      .getWinner()
      .expect("Impossible: No winner but no nextStates");
  }

  function updateAndBackPropagate(leaf: Node, winner: Player): void {
    let node = leaf;
    let parent = node.edgeConnectingToParent?.parent;

    while (parent !== undefined) {
      node.value += winner === parent.state.turn ? 1 : 0;
      node.rollouts += 1;

      node = parent;
      parent = node.edgeConnectingToParent?.parent;
    }

    node.value += winner === node.state.turn ? 1 : 0;
    node.rollouts += 1;
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
