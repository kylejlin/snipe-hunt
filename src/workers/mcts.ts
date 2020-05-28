import { option, Option } from "rusty-ts";
import { getAnalyzer } from "../analyzer";
import { getMctsUtils, MctsAnalyzer } from "../mcts";
import {
  GameState,
  MctsWorkerMessage,
  UpdateMctsAnalysisNotification,
  UpdateMctsAnalyzerGameStateRequest,
  WorkerMessageType,
} from "../types";

export {};

const ROLLOUT_BATCH_SIZE = 5000;
const UNCERTAINTY_THRESHOLD = 1e-3;
const MAX_MEAN_VALUE_BEFORE_DECLARING_VICTORY = 1 - UNCERTAINTY_THRESHOLD;
const MIN_MEAN_VALUE_BEFORE_DECLARING_DEFEAT = UNCERTAINTY_THRESHOLD;
const MIN_ROLLOUTS_NEEDED_TO_DECLARE_STATE_TERMINAL = 1e6;
const MIN_MILLISECONDS_BETWEEN_POSTS = 0.2e3;

declare const self: Worker;

let optMctsAnalyzer: Option<MctsAnalyzer> = option.none();
let lastPosted = 0;

self.addEventListener("message", (e) => {
  const data = e.data;
  if ("object" === typeof data && data !== null) {
    {
      if (data.x) {
        self.postMessage({
          messageType: WorkerMessageType.LogNotification,
          data: optMctsAnalyzer.unwrap().getRoot(),
        });
      }
    }
    const message: MctsWorkerMessage = data;
    switch (message.messageType) {
      case WorkerMessageType.UpdateMctsAnalyzerGameStateRequest:
        onGameStateUpdateRequest(message);
        break;
    }
  }
});

function onGameStateUpdateRequest(
  message: UpdateMctsAnalyzerGameStateRequest
): void {
  optMctsAnalyzer = getMctsAnalyzer(message.gameState);
}

function getMctsAnalyzer(gameState: GameState): Option<MctsAnalyzer> {
  const gameAnalyzer = getAnalyzer(gameState);
  if (gameAnalyzer.isGameOver()) {
    return option.none();
  }

  const mctsAnalyzer = getMctsUtils(gameState, getAnalyzer(gameState));
  mctsAnalyzer.performRollout();
  mctsAnalyzer.performRollout();
  return option.some(mctsAnalyzer);
}

function analysisUpdateLoop() {
  optMctsAnalyzer.match({
    none: () => {
      const message: UpdateMctsAnalysisNotification = {
        messageType: WorkerMessageType.UpdateMctsAnalysisNotification,
        optAnalysis: null,
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

function postAnalysisUpdate(analyzer: MctsAnalyzer): void {
  const [bestAtomic, childWithBestAtomic] = option
    .all([analyzer.getBestAtomic(), analyzer.getChildWithBestAtomic()])
    .expect("Impossible: optMctsAnalyzer is some when game has already eneded");
  const message: UpdateMctsAnalysisNotification = {
    messageType: WorkerMessageType.UpdateMctsAnalysisNotification,
    optAnalysis: {
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
