import { option, Option } from "rusty-ts";
import {
  getMctsAnalyzerIfStateIsNonTerminal,
  MctsAnalyzer,
  NODE_SIZE_IN_I32S,
  getMctsAnalyzerFromInternalDataWithoutInitializing,
} from "../mcts";
import {
  MctsWorkerRequest,
  UpdateSnapshotNotification,
  UpdateGameStateRequest,
  MctsWorkerMessageType,
  PauseAnalyzerRequest,
  PauseAnalyzerResponse,
  ResumeAnalyzerRequest,
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
let optStopTime: Option<number> = option.none();

self.addEventListener("message", (e) => {
  const data = e.data;
  if ("object" === typeof data && data !== null) {
    const message: MctsWorkerRequest = data;
    switch (message.messageType) {
      case MctsWorkerMessageType.UpdateGameStateRequest:
        onGameStateUpdateRequest(message);
        break;
      case MctsWorkerMessageType.PauseAnalyzerRequest:
        onPauseAnalyzerRequest(message);
        break;
      case MctsWorkerMessageType.ResumeAnalyzerRequest:
        onResumeAnalyzerRequest(message);
        break;
      default: {
        // Force exhaustive matching

        // eslint-disable-next-line
        const unreachable: never = message;
      }
    }
  }
});

function onGameStateUpdateRequest(message: UpdateGameStateRequest): void {
  const newGameState = message.gameState;

  optMctsAnalyzer = getMctsAnalyzerIfStateIsNonTerminal(
    newGameState,
    newGameState.plies.length + 3,
    NODE_SIZE_IN_I32S * 2e7
  );

  if (Number.isFinite(message.thinkingTimeInMS)) {
    optStopTime = option.some(Date.now() + message.thinkingTimeInMS);
  } else {
    optStopTime = option.none();
  }
}

function onPauseAnalyzerRequest(_message: PauseAnalyzerRequest): void {
  optMctsAnalyzer.ifSome((mctsAnalyzer) => {
    const internalData = mctsAnalyzer.getInternalData();
    optMctsAnalyzer = option.none();

    const message: PauseAnalyzerResponse = {
      messageType: MctsWorkerMessageType.PauseAnalyzerResponse,
      internalData,
    };
    self.postMessage(message, [internalData.heapBuffer]);
  });
}

function onResumeAnalyzerRequest(message: ResumeAnalyzerRequest): void {
  optMctsAnalyzer = option.some(
    getMctsAnalyzerFromInternalDataWithoutInitializing(message.internalData)
  );
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
      const root = analyzer.getNodeSummary(analyzer.getRootPointer());
      const meanValue = root.value / root.rollouts;
      const isTerminal =
        root.rollouts >= MIN_ROLLOUTS_NEEDED_TO_DECLARE_STATE_TERMINAL &&
        (meanValue > MAX_MEAN_VALUE_BEFORE_DECLARING_VICTORY ||
          meanValue < MIN_MEAN_VALUE_BEFORE_DECLARING_DEFEAT);
      const isOutOfThinkingTime = optStopTime.someSatisfies(
        (stopTime) => Date.now() > stopTime
      );

      if (!isTerminal && !isOutOfThinkingTime) {
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
  const message: UpdateSnapshotNotification = {
    messageType: MctsWorkerMessageType.UpdateSnapshotNotification,
    optSnapshot: analyzer.getSnapshot(),
  };
  self.postMessage(message);
}

analysisUpdateLoop();
