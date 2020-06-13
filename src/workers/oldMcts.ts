import { option, Option } from "rusty-ts";
import { getMinimalGameStateAnalyzer } from "../minimalAnalyzer";
import {
  getMctsAnalyzerForNonTerminalStateFromRoot,
  getMctsAnalyzerIfStateIsNonTerminal,
  OldMctsAnalyzer,
} from "../oldMcts";
import {
  MctsWorkerMessageType,
  MctsWorkerRequest,
  UpdateGameStateRequest,
  UpdateSnapshotNotification,
} from "../types";

export {};

const ROLLOUT_BATCH_SIZE = 5000;
const UNCERTAINTY_THRESHOLD = 1e-3;
const MAX_MEAN_VALUE_BEFORE_DECLARING_VICTORY = 1 - UNCERTAINTY_THRESHOLD;
const MIN_MEAN_VALUE_BEFORE_DECLARING_DEFEAT = UNCERTAINTY_THRESHOLD;
const MIN_ROLLOUTS_NEEDED_TO_DECLARE_STATE_TERMINAL = 1e6;
const MIN_MILLISECONDS_BETWEEN_POSTS = 0.2e3;

declare const self: Worker;

let optMctsAnalyzer: Option<OldMctsAnalyzer> = option.none();
let lastPosted = 0;

self.addEventListener("message", (e) => {
  const data = e.data;
  if ("object" === typeof data && data !== null) {
    {
      if (data.x) {
        self.postMessage({
          messageType: MctsWorkerMessageType.LogNotification,
          data: optMctsAnalyzer.unwrap().getRoot(),
        });
      }
    }
    const message: MctsWorkerRequest = data;
    switch (message.messageType) {
      case MctsWorkerMessageType.UpdateGameStateRequest:
        onGameStateUpdateRequest(message);
        break;
    }
  }
});

function onGameStateUpdateRequest(message: UpdateGameStateRequest): void {
  const newGameState = message.gameState;

  const optRecycledAnalyzer = optMctsAnalyzer.andThen((mctsAnalyzer) => {
    const selectedChild = mctsAnalyzer
      .getRoot()
      .children.find(
        (child) =>
          getMinimalGameStateAnalyzer(child.state).toNodeKey() ===
          getMinimalGameStateAnalyzer(newGameState).toNodeKey()
      );

    if (selectedChild !== undefined) {
      if (
        selectedChild.edgeConnectingToParent!.parent.state.turn !==
        selectedChild.state.turn
      ) {
        selectedChild.value = selectedChild.rollouts - selectedChild.value;
      }

      selectedChild.edgeConnectingToParent = undefined;

      return option.some(
        getMctsAnalyzerForNonTerminalStateFromRoot(
          selectedChild,
          getMinimalGameStateAnalyzer(selectedChild.state)
        )
      );
    } else {
      return option.none();
    }
  });

  optMctsAnalyzer = optRecycledAnalyzer.orElse(() => {
    return getMctsAnalyzerIfStateIsNonTerminal(newGameState);
  });
}

function analysisUpdateLoop() {
  optMctsAnalyzer.match({
    none: () => {
      const message: UpdateSnapshotNotification = {
        messageType: MctsWorkerMessageType.UpdateSnapshotNotification,
        optSnapshot: null,
      };
      self.postMessage(message);
    },

    some: (analyzer) => {
      const root = analyzer.getRoot();
      const meanValue = root.value / root.rollouts;
      const isTerminal =
        root.rollouts >= MIN_ROLLOUTS_NEEDED_TO_DECLARE_STATE_TERMINAL &&
        (meanValue > MAX_MEAN_VALUE_BEFORE_DECLARING_VICTORY ||
          meanValue < MIN_MEAN_VALUE_BEFORE_DECLARING_DEFEAT);

      if (!isTerminal) {
        for (let i = 0; i < ROLLOUT_BATCH_SIZE; i++) {
          analyzer.performRollout();
        }
      }

      const now = Date.now();
      const shouldPost = now > lastPosted + MIN_MILLISECONDS_BETWEEN_POSTS;
      if (shouldPost) {
        postAnalysisUpdate(analyzer);
        lastPosted = now;
      }
    },
  });

  requestAnimationFrame(analysisUpdateLoop);
}

function postAnalysisUpdate(analyzer: OldMctsAnalyzer): void {
  const [bestAtomic, childWithBestAtomic] = option
    .all([analyzer.getBestAtomic(), analyzer.getChildWithBestAtomic()])
    .expect("Impossible: optMctsAnalyzer is some when game has already eneded");
  const message: UpdateSnapshotNotification = {
    messageType: MctsWorkerMessageType.UpdateSnapshotNotification,
    optSnapshot: {
      currentStateValue: analyzer.getRoot().value,
      currentStateRollouts: analyzer.getRoot().rollouts,

      bestAtomic,
      bestAtomicValue: childWithBestAtomic.value,
      bestAtomicRollouts: childWithBestAtomic.rollouts,
    },
  };
  self.postMessage(message);
}

analysisUpdateLoop();
