import { option, Option } from "rusty-ts";
import { getAnalyzer } from "../analyzer";
import { getMctsUtils, MctsUtils } from "../mcts";
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
const MILLISECONDS_BETWEEN_TERMINAL_POSTS = 0.1e3;

declare const self: Worker;

let optMctsAnalyzer: Option<MctsUtils> = option.none();
let lastPosted = 0;

self.addEventListener("message", (e) => {
  const data = e.data;
  if ("object" === typeof data && data !== null) {
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

function getMctsAnalyzer(gameState: GameState): Option<MctsUtils> {
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
      if (isTerminal) {
        const now = Date.now();
        if (now > lastPosted + MILLISECONDS_BETWEEN_TERMINAL_POSTS) {
          postAnalysisUpdate(analyzer);
        }
        lastPosted = now;
      } else {
        for (let i = 0; i < ROLLOUT_BATCH_SIZE; i++) {
          analyzer.performRollout();
        }
        postAnalysisUpdate(analyzer);
      }
    },
  });

  requestAnimationFrame(analysisUpdateLoop);
}

function postAnalysisUpdate(analyzer: MctsUtils): void {
  const message: UpdateMctsAnalysisNotification = {
    messageType: WorkerMessageType.UpdateMctsAnalysisNotification,
    optAnalysis: {
      bestAtomic: analyzer
        .getBestAtomic()
        .expect(
          "Impossible: optMctsAnalyzer is some when game has already eneded"
        ),
      value: analyzer.getRoot().value,
      rollouts: analyzer.getRoot().rollouts,
    },
  };
  self.postMessage(message);
}

analysisUpdateLoop();
