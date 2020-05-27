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
    let parentRollouts = 0;
    let leaf = root;
    while (!isLeaf(leaf)) {
      const bestChild = select(leaf.children, parentRollouts);
      parentRollouts = leaf.rollouts;
      leaf = bestChild;
    }

    if (leaf.rollouts === 0) {
      rolloutOrMarkAsTerminalThenBackPropagate(leaf);
    } else {
      const children = getChildren(leaf);
      rolloutOrMarkAsTerminalThenBackPropagate(children[0]);
      leaf.children = children;
    }
  }

  function isLeaf(node: Node): boolean {
    return node.children.length === 0;
  }

  function select(nodes: Node[], parentRollouts: number): Node {
    let maxScore = getScore(nodes[0], parentRollouts);
    let bestNode = nodes[0];

    for (let i = 1; i < nodes.length; i++) {
      const node = nodes[i];
      const score = getScore(node, parentRollouts);
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
  function getScore(node: Node, parentRollouts: number) {
    if (node.rollouts === 0 || parentRollouts === 0) {
      return Infinity;
    }

    const v = node.value / node.rollouts;
    return (
      v +
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
            valueIncrease: BIG_NUMBER - node.value,
            rolloutIncrease: BIG_NUMBER - node.value,
          };
        } else {
          return {
            valueIncrease: -BIG_NUMBER - node.value,
            rolloutIncrease: -BIG_NUMBER - node.value,
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
