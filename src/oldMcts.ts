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
import { opponentOf } from "./gameUtil";

export interface OldMctsAnalyzer {
  performRollout(): void;
  getRoot(): OldMctsNode;
  getBestAtomic(): Option<Atomic>;
  getChildWithBestAtomic(): Option<OldMctsNode>;
}

export interface OldMctsNode {
  edgeConnectingToParent: EdgeConnectingToParent | undefined;
  state: MinimalGameState;
  effectiveWinner: Player | undefined;
  value: number;
  rollouts: number;
  children: OldMctsNode[];
}

interface EdgeConnectingToParent {
  parent: OldMctsNode;
  atomic: Atomic;
}

const EXPLORATION_CONSTANT = Math.sqrt(2);

export function getMctsAnalyzerIfStateIsNonTerminal(
  state: GameState
): Option<OldMctsAnalyzer> {
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
): OldMctsAnalyzer {
  const root: OldMctsNode = {
    edgeConnectingToParent: undefined,
    state,
    effectiveWinner: undefined,
    value: 0,
    rollouts: 0,
    children: [],
  };
  return getMctsAnalyzerForNonTerminalStateFromRoot(root, analyzer);
}

export function getMctsAnalyzerForNonTerminalStateFromRoot(
  root: OldMctsNode,
  analyzer: MinimalGameAnalyzer
): OldMctsAnalyzer {
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
          updateAndBackPropagateRollout(leaf, winner);
          markAsTerminal(leaf, winner);
        },
        none: () => {
          const children = getChildren(leaf);
          rolloutIfNonTerminalThenBackPropagate(children[0]);
          leaf.children = children;
        },
      });
    }
  }

  function isLeaf(node: OldMctsNode): boolean {
    return node.children.length === 0;
  }

  function selectBestChildAccordingToActivePlayer(
    node: OldMctsNode
  ): OldMctsNode {
    const activePlayer = node.state.turn;
    const activePlayerOpponent = opponentOf(activePlayer);

    const { children } = node;

    let maxScore = getUcbScore(children[0], node.rollouts);
    let bestNode = children[0];

    if (bestNode.effectiveWinner === activePlayer) {
      return bestNode;
    }

    for (let i = 1; i < children.length; i++) {
      const child = children[i];

      if (child.effectiveWinner === activePlayer) {
        return child;
      }

      const score = getUcbScore(child, node.rollouts);
      if (
        score > maxScore ||
        (bestNode.effectiveWinner === activePlayerOpponent &&
          child.effectiveWinner !== activePlayerOpponent)
      ) {
        maxScore = score;
        bestNode = child;
      }
    }

    if (bestNode.effectiveWinner === activePlayerOpponent) {
      markAsTerminal(node, activePlayerOpponent);
    }

    return bestNode;
  }

  /**
   * Upper confidence bound (UCB1) based on
   * https://www.youtube.com/watch?v=UXW2yZndl7U
   */
  function getUcbScore(node: OldMctsNode, parentRollouts: number) {
    if (node.rollouts === 0 || parentRollouts === 0) {
      return Infinity;
    }

    const meanValue = node.value / node.rollouts;
    return (
      meanValue +
      EXPLORATION_CONSTANT * Math.sqrt(Math.log(parentRollouts) / node.rollouts)
    );
  }

  function rolloutIfNonTerminalThenBackPropagate(node: OldMctsNode): void {
    analyzer.setState(node.state);
    const optWinner = analyzer.getWinner();

    optWinner.ifSome((winner) => {
      markAsTerminal(node, winner);
    });

    const winner = optWinner.unwrapOrElse(() => rollout(node.state));

    updateAndBackPropagateRollout(node, winner);
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

  function updateAndBackPropagateRollout(
    leaf: OldMctsNode,
    winner: Player
  ): void {
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

  function markAsTerminal(leaf: OldMctsNode, winner: Player): void {
    let node = leaf;
    let parent = node.edgeConnectingToParent?.parent;

    while (parent !== undefined && parent.state.turn === winner) {
      node.effectiveWinner = winner;

      node = parent;
      parent = node.edgeConnectingToParent?.parent;
    }

    node.effectiveWinner = winner;
  }

  function getChildren(node: OldMctsNode): OldMctsNode[] {
    analyzer.setState(node.state);
    return analyzer.getLegalAtomics().map((atomic) => ({
      edgeConnectingToParent: {
        parent: node,
        atomic,
      },
      state: analyzer.forcePerform(atomic),
      effectiveWinner: undefined,
      value: 0,
      rollouts: 0,
      children: [],
    }));
  }

  function getRoot(): OldMctsNode {
    return root;
  }

  function getBestAtomic(): Option<Atomic> {
    return getChildWithBestAtomic().map(
      (child) => child.edgeConnectingToParent!.atomic
    );
  }

  function getChildWithBestAtomic(): Option<OldMctsNode> {
    let bestChild: OldMctsNode | undefined = undefined;
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
