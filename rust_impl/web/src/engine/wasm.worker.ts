/// <reference lib="webworker" />

import init, { analyze, analyze_live } from "../wasm/pkg/snipe_wasm.js";
import { isWasmMemoryLimitTrap } from "./wasm-errors";
import type { WorkerRequest, WorkerResponse } from "./worker-protocol";
import type { AnalysisResult, LiveAnalysisUpdate } from "./types";

const scope = self as DedicatedWorkerGlobalScope;
const ready = init();

scope.onmessage = async (event: MessageEvent<WorkerRequest>) => {
  const message = event.data;
  if (message.type === "cancel") return;
  let runtime: Awaited<typeof ready> | null = null;
  try {
    runtime = await ready;
    // Rust search is intentionally synchronous here. Cancellation is performed
    // by terminating this worker, since a busy worker cannot receive messages.
    if (message.type === "agent") {
      const result = JSON.parse(analyze(JSON.stringify(message.payload))) as AnalysisResult;
      const response: WorkerResponse = { type: "agent-result", payload: result };
      scope.postMessage(response);
    } else {
      const final = JSON.parse(
        analyze_live(JSON.stringify(message.payload), (json: string) => {
          const payload = JSON.parse(json) as LiveAnalysisUpdate;
          const progress: WorkerResponse = { type: "analysis-progress", payload };
          scope.postMessage(progress);
        }),
      ) as LiveAnalysisUpdate;
      const response: WorkerResponse = { type: "analysis-complete", payload: final };
      scope.postMessage(response);
    }
  } catch (reason) {
    const response: WorkerResponse = {
      type: "error",
      requestId: message.payload.requestId,
      message: reason instanceof Error ? reason.message : String(reason),
      ...(isWasmMemoryLimitTrap(
        reason,
        runtime?.memory.buffer.byteLength ?? 0,
      )
        ? { code: "memory-limit" as const }
        : {}),
    };
    scope.postMessage(response);
  }
};

export {};
