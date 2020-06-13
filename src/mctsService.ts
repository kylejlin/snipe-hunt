import { option } from "rusty-ts";
import {
  getMctsAnalyzerFromInternalDataWithoutInitializing,
  MctsAnalyzer,
} from "./mcts";
import {
  GameState,
  LogNotification,
  MctsService,
  MctsWorkerMessageType,
  PauseAnalyzerRequest,
  PauseAnalyzerResponse,
  UpdateGameStateRequest,
  UpdateSnapshotNotification,
  MctsWorkerNotification,
  ResumeAnalyzerRequest,
} from "./types";
import MctsWorker from "./workers/mcts.importable";

type SnapshotListener = Parameters<MctsService["onSnapshot"]>[0];
type PauseListener = Parameters<MctsService["onPause"]>[0];

export function getMctsService(): MctsService {
  const worker = new MctsWorker();

  const snapshotListeners: SnapshotListener[] = [];
  const pauseListeners: PauseListener[] = [];

  addWorkerListener();

  return {
    updateGameState,
    pause,
    resume,

    onSnapshot,
    onPause,
  };

  function addWorkerListener(): void {
    worker.addEventListener("message", onWorkerMessage);
  }

  function onWorkerMessage(event: MessageEvent): void {
    const { data } = event;
    if (data !== null && "object" === typeof data) {
      const message: MctsWorkerNotification = data;
      switch (message.messageType) {
        case MctsWorkerMessageType.LogNotification:
          onWorkerLogNotification(message);
          break;
        case MctsWorkerMessageType.UpdateSnapshotNotification:
          onUpdateSnapshotNotification(message);
          break;
        case MctsWorkerMessageType.PauseAnalyzerResponse:
          onTransferAnalyzerResponse(message);
          break;
        default: {
          // Force exhaustive matching

          // eslint-disable-next-line
          const unreachable: never = message;
        }
      }
    }
  }

  function onWorkerLogNotification(message: LogNotification): void {
    console.log("MCTS Worker Log:", message.data);
  }

  function onUpdateSnapshotNotification(
    message: UpdateSnapshotNotification
  ): void {
    const optSnapshot = option.fromVoidable(message.optAnalysis);
    for (const listener of snapshotListeners) {
      listener(optSnapshot);
    }
  }

  function onTransferAnalyzerResponse(message: PauseAnalyzerResponse): void {
    const analyzer = getMctsAnalyzerFromInternalDataWithoutInitializing(
      message.internalData
    );
    for (const listener of pauseListeners) {
      listener(analyzer);
    }
  }

  function updateGameState(state: GameState): void {
    const message: UpdateGameStateRequest = {
      messageType: MctsWorkerMessageType.UpdateGameStateRequest,
      gameState: state,
    };
    worker.postMessage(message);
  }

  function pause(): void {
    const message: PauseAnalyzerRequest = {
      messageType: MctsWorkerMessageType.PauseAnalyzerRequest,
    };
    worker.postMessage(message);
  }

  function resume(analyzer: MctsAnalyzer): void {
    const internalData = analyzer.getInternalData();
    const message: ResumeAnalyzerRequest = {
      messageType: MctsWorkerMessageType.ResumeAnalyzerRequest,
      internalData,
    };
    worker.postMessage(message, [internalData.heapBuffer]);
  }

  function onSnapshot(listener: SnapshotListener): void {
    snapshotListeners.push(listener);
  }

  function onPause(listener: PauseListener): void {
    pauseListeners.push(listener);
  }
}
