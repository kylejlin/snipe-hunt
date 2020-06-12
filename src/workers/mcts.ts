import { option, Option } from "rusty-ts";
import {
  getMctsAnalyzerIfStateIsNonTerminal,
  MctsAnalyzer,
  NODE_SIZE_IN_I32S,
} from "../mcts";
import {
  MctsWorkerMessage,
  UpdateSnapshotNotification,
  UpdateGameStateRequest,
  MctsWorkerMessageType,
  TransferAnalyzerRequest,
  TransferAnalyzerResponse,
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
    const message: MctsWorkerMessage = data;
    switch (message.messageType) {
      case MctsWorkerMessageType.UpdateGameStateRequest:
        onGameStateUpdateRequest(message);
        break;
      case MctsWorkerMessageType.TransferAnalyzerRequest:
        onTransferHeapRequest(message);
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
}

function onTransferHeapRequest(_message: TransferAnalyzerRequest): void {
  optMctsAnalyzer.ifSome((mctsAnalyzer) => {
    const internalData = mctsAnalyzer.getInternalData();
    optMctsAnalyzer = option.none();

    const message: TransferAnalyzerResponse = {
      messageType: MctsWorkerMessageType.TransferAnalyzerResponse,
      internalData,
    };
    self.postMessage(message, [internalData.heapBuffer]);
  });
}

function analysisUpdateLoop() {
  optMctsAnalyzer.match({
    none: () => {
      const message: UpdateSnapshotNotification = {
        messageType: MctsWorkerMessageType.UpdateSnapshotNotification,
        optAnalysis: null,
      };
      self.postMessage(message);
    },

    some: (analyzer) => {
      const root = analyzer.getRootSummary();
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
  const root = analyzer.getRootSummary();
  const bestAtomic = analyzer.getBestAtomic();
  const childWithBestAtomic = analyzer.getSummaryOfChildWithBestAtomic();

  const message: UpdateSnapshotNotification = {
    messageType: MctsWorkerMessageType.UpdateSnapshotNotification,
    optAnalysis: {
      currentStateValue: root.value,
      currentStateRollouts: root.rollouts,

      bestAtomic,
      bestAtomicValue: childWithBestAtomic.value,
      bestAtomicRollouts: childWithBestAtomic.rollouts,
    },
  };
  self.postMessage(message);
}

analysisUpdateLoop();
