/// <reference lib="webworker" />
import { analyzeFallback, analyzeFallbackAtDepth } from "./fallback-core";
import type { WorkerRequest, WorkerResponse } from "./worker-protocol";

const cancelled = new Set<number>();

self.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const message = event.data;
  if (message.type === "cancel") {
    cancelled.add(message.requestId);
    return;
  }

  const { requestId, position } = message.payload;
  if (message.type === "analysis") {
    let depth = 1;
    const tick = () => {
      if (cancelled.delete(requestId)) return;
      try {
        const payload = analyzeFallbackAtDepth(
          position,
          requestId,
          depth,
          message.payload.firstStep,
        );
        const response: WorkerResponse =
          depth >= message.payload.maxDepth
            ? { type: "analysis-complete", payload }
            : { type: "analysis-progress", payload };
        self.postMessage(response);
        depth += 1;
        if (depth <= message.payload.maxDepth) setTimeout(tick, 45);
      } catch (error) {
        const response: WorkerResponse = {
          type: "error",
          requestId,
          message: error instanceof Error ? error.message : "Analysis failed.",
        };
        self.postMessage(response);
      }
    };
    setTimeout(tick, 20);
    return;
  }

  const { timeLimitMs } = message.payload;
  const started = performance.now();
  const simulatedBudget = Math.max(90, Math.min(420, timeLimitMs * 0.08));

  const finish = () => {
    if (cancelled.delete(requestId)) return;
    try {
      const elapsedMs = Math.round(performance.now() - started);
      const response: WorkerResponse = {
        type: "agent-result",
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
