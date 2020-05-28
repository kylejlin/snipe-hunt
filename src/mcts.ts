import { GameAnalyzer, GameState } from "./types";

export interface MctsUtils {
  performCycle(): void;
}

export interface Node {
  parent: Node | undefined;
  state: GameState;
  value: number;
  rollouts: number;
  children: Node[];
}

const EXPLORATION_CONSTANT = 2;
const BIG_NUMBER = 1e7;

export function getMctsUtils(
  state: GameState,
  analyzer: GameAnalyzer
): MctsUtils {
  const root: Node = {
    parent: undefined,
    state,
    value: 0,
    rollouts: 0,
    children: [],
  };

  analyzer.setState(state);
  const perspective = analyzer.getTurn();

  return { performCycle };

  function performCycle(): void {
    let node = root;
    while (!isLeaf(node)) {
      const bestChild = selectBestChildAccordingToActivePlayer(node);
      node = bestChild;
    }
    const leaf = node;

    if (leaf.rollouts === 0) {
      rolloutOrMarkAsTerminalThenBackPropagate(node);
    } else {
      const children = getChildren(node);
      rolloutOrMarkAsTerminalThenBackPropagate(children[0]);
      node.children = children;
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
      const node = children[i];
      const score = getScore(node, node.rollouts, invertMeanValue);
      if (score > maxScore) {
        maxScore = score;
        bestNode = node;
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
    const meanValue = invertMeanValue ? 1 - rawMeanValue : rawMeanValue;
    return (
      meanValue +
      EXPLORATION_CONSTANT * Math.sqrt(Math.log(parentRollouts) / node.rollouts)
    );
  }

  function rolloutOrMarkAsTerminalThenBackPropagate(node: Node): void {
    analyzer.setState(node.state);
    const winner = analyzer.getWinner();

    const { valueIncrease, rolloutIncrease } = winner.match({
      some: (winner) => {
        if (winner === perspective) {
          return {
            valueIncrease: BIG_NUMBER,
            rolloutIncrease: BIG_NUMBER,
          };
        } else {
          return {
            valueIncrease: 0,
            rolloutIncrease: BIG_NUMBER,
          };
        }
      },

      none: () => {
        const valueIncrease = rollout(node.state);
        return { valueIncrease, rolloutIncrease: 1 };
      },
    });

    updateAndBackPropagate(node, valueIncrease, rolloutIncrease);
  }

  function rollout(state: GameState): 0 | 1 {
    analyzer.setState(state);

    let nextStates = analyzer.getStatesAfterPerformingOneAtomic();
    while (nextStates.length > 0) {
      const selected = nextStates[randInt(0, nextStates.length)];
      analyzer.setState(selected);
      nextStates = analyzer.getStatesAfterPerformingOneAtomic();
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

  function randInt(inclMin: number, exclMax: number): number {
    const diff = exclMax - inclMin;
    return inclMin + Math.floor(diff * Math.random());
  }

  function updateAndBackPropagate(
    leaf: Node,
    valueIncrease: number,
    rolloutIncrease: number
  ): void {
    let node: Node | undefined = leaf;
    while (node !== undefined) {
      node.value += valueIncrease;
      node.rollouts += rolloutIncrease;
      node = leaf.parent;
    }
  }

  function getChildren(node: Node): Node[] {
    analyzer.setState(node.state);
    return analyzer.getStatesAfterPerformingOneAtomic().map((state) => ({
      parent: node,
      state,
      value: 0,
      rollouts: 0,
      children: [],
    }));
  }
}
