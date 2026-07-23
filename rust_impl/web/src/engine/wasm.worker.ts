/// <reference lib="webworker" />

import init, { analyze } from "../wasm/pkg/snipe_wasm.js";
import type { WorkerRequest, WorkerResponse } from "./worker-protocol";
import type { AnalysisResult } from "./types";

const scope = self as DedicatedWorkerGlobalScope;
const ready = init();

scope.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const message = event.data;
  if (message.type !== "analyze") return;
  try {
    await ready;
    // Rust search is intentionally synchronous here. Cancellation is performed
    // by terminating this worker, since a busy worker cannot receive messages.
    const result = JSON.parse(analyze(JSON.stringify(message.payload))) as AnalysisResult;
    const response: WorkerResponse = { type: "result", payload: result };
    scope.postMessage(response);
  } catch (reason) {
    const response: WorkerResponse = {
      type: "error",
      requestId: message.payload.requestId,
      message: reason instanceof Error ? reason.message : String(reason),
    };
    scope.postMessage(response);
  }
};

export {};
