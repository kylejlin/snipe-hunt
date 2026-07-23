/// <reference lib="webworker" />
import { analyzeFallback } from "./fallback-core";
import type { WorkerRequest, WorkerResponse } from "./worker-protocol";

const cancelled = new Set<number>();

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const message = event.data;
  if (message.type === "cancel") {
    cancelled.add(message.requestId);
    return;
  }

  const { requestId, position, timeLimitMs } = message.payload;
  const started = performance.now();
  const simulatedBudget = Math.max(90, Math.min(420, timeLimitMs * 0.08));

  const finish = () => {
    if (cancelled.delete(requestId)) return;
    try {
      const elapsedMs = Math.round(performance.now() - started);
      const response: WorkerResponse = {
        type: "result",
        payload: analyzeFallback(position, requestId, elapsedMs),
      };
      self.postMessage(response);
    } catch (error) {
      const response: WorkerResponse = {
        type: "error",
        requestId,
        message: error instanceof Error ? error.message : "Analysis failed.",
      };
      self.postMessage(response);
    }
  };

  setTimeout(finish, simulatedBudget);
};
