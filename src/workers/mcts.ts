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

declare const self: Worker;

let optMctsAnalyzer: Option<MctsUtils> = option.none();

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
      for (let i = 0; i < ROLLOUT_BATCH_SIZE; i++) {
        analyzer.performRollout();
      }
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
    },
  });
  requestAnimationFrame(analysisUpdateLoop);
}

analysisUpdateLoop();
