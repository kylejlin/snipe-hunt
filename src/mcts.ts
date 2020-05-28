import { Option, option } from "rusty-ts";
import randInt from "./randInt";
import { Atomic, GameAnalyzer, GameState } from "./types";

export interface MctsAnalyzer {
  performRollout(): void;
  getRoot(): Node;
  getBestAtomic(): Option<Atomic>;
  getChildWithBestAtomic(): Option<Node>;
}

export interface Node {
  edgeConnectingToParent: EdgeConnectingToParent | undefined;
  state: GameState;
  value: number;
  rollouts: number;
  children: Node[];
}

interface EdgeConnectingToParent {
  parent: Node;
  atomic: Atomic;
}

const EXPLORATION_CONSTANT = Math.sqrt(2);

export function getMctsUtils(
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
  const perspective = analyzer.getTurn();

  return { performRollout, getRoot, getBestAtomic, getChildWithBestAtomic };

  function performRollout(): void {
    let node = root;
    while (!isLeaf(node)) {
      const bestChild = selectBestChildAccordingToActivePlayer(node);
      node = bestChild;
    }
    const leaf = node;

    if (leaf.rollouts === 0) {
      rolloutIfNonTerminalThenBackPropagate(leaf);
    } else {
      analyzer.setState(leaf.state);
      analyzer.getWinner().match({
        some: (winner) => {
          const valueIncrease = winner === perspective ? 1 : 0;
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
    const invertMeanValue = node.state.turn !== perspective;
    const { children } = node;

    let maxScore = getScore(children[0], node.rollouts, invertMeanValue);
    let bestNode = children[0];

    for (let i = 1; i < children.length; i++) {
      const child = children[i];
      const score = getScore(child, node.rollouts, invertMeanValue);
      if (score > maxScore) {
        maxScore = score;
        bestNode = child;
      }
    }

    return bestNode;
  }

  /**
   * Upper confidence bound (UCB1) as described in
   * https://www.youtube.com/watch?v=UXW2yZndl7U
   */
  function getScore(
    node: Node,
    parentRollouts: number,
    invertMeanValue: boolean
  ) {
    if (node.rollouts === 0 || parentRollouts === 0) {
      return Infinity;
    }

    const rawMeanValue = node.value / node.rollouts;
    const adjustedMeanValue = invertMeanValue ? 1 - rawMeanValue : rawMeanValue;
    return (
      adjustedMeanValue +
      EXPLORATION_CONSTANT * Math.sqrt(Math.log(parentRollouts) / node.rollouts)
    );
  }

  function rolloutIfNonTerminalThenBackPropagate(node: Node): void {
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
        return rollout(node.state);
      },
    });

    updateAndBackPropagate(node, valueIncrease);
  }

  function rollout(state: GameState): 0 | 1 {
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
