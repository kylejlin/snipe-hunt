import {
  analyzeFallback,
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
  previewFallbackFirstStep,
} from "./fallback-core";
import type { WorkerRequest, WorkerResponse } from "./worker-protocol";
import type {
  AnalysisRequest,
  AnalysisResult,
  EngineAdapter,
  MoveStep,
  Position,
  TurnMove,
} from "./types";
import { WasmEngineAdapter } from "./wasm-adapter";
import { wasmInitializationError, wasmReady } from "./wasm-runtime";

interface PendingAnalysis {
  resolve: (result: AnalysisResult) => void;
  reject: (reason: Error) => void;
}

export class FallbackEngineAdapter implements EngineAdapter {
  readonly name = "Deterministic preview engine";
  private worker: Worker | null = null;
  private pending = new Map<number, PendingAnalysis>();

  constructor() {
    if (typeof Worker !== "undefined") {
      this.worker = new Worker(new URL("./fallback.worker.ts", import.meta.url), { type: "module" });
      this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
        const message = event.data;
        const requestId = message.type === "result" ? message.payload.requestId : message.requestId;
        const pending = this.pending.get(requestId);
        if (!pending) return;
        this.pending.delete(requestId);
        if (message.type === "result") pending.resolve(message.payload);
        else pending.reject(new Error(message.message));
      };
    }
  }

  createGame(seed?: number): Position {
    return createFallbackGame(seed);
  }

  legalMoves(position: Position): TurnMove[] {
    return fallbackLegalMoves(position);
  }

  previewFirstStep(position: Position, step: MoveStep): Position {
    return previewFallbackFirstStep(position, step);
  }

  applyMove(position: Position, move: TurnMove): Position {
    return applyFallbackMove(position, move);
  }

  analyze(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult> {
    if (!this.worker) {
      return new Promise((resolve, reject) => {
        const timer = window.setTimeout(
          () => resolve(analyzeFallback(request.position, request.requestId, 120)),
          120,
        );
        signal.addEventListener(
          "abort",
          () => {
            clearTimeout(timer);
            reject(new DOMException("Analysis cancelled.", "AbortError"));
          },
          { once: true },
        );
      });
    }

    return new Promise((resolve, reject) => {
      const abort = () => {
        const cancel: WorkerRequest = { type: "cancel", requestId: request.requestId };
        this.worker?.postMessage(cancel);
        this.pending.delete(request.requestId);
        reject(new DOMException("Analysis cancelled.", "AbortError"));
      };
      signal.addEventListener("abort", abort, { once: true });
      this.pending.set(request.requestId, {
        resolve: (result) => {
          signal.removeEventListener("abort", abort);
          resolve(result);
        },
        reject,
      });
      const message: WorkerRequest = { type: "analyze", payload: request };
      this.worker?.postMessage(message);
    });
  }

  dispose(): void {
    this.worker?.terminate();
    this.worker = null;
    for (const pending of this.pending.values()) pending.reject(new Error("Engine disposed."));
    this.pending.clear();
  }
}

export function createEngineAdapter(): EngineAdapter {
  if (wasmReady) {
    try {
      return new WasmEngineAdapter();
    } catch (reason) {
      console.warn(
        "Snipe Hunt Rust/WASM worker failed to initialize; using the deterministic preview engine.",
        reason,
      );
      return new FallbackEngineAdapter();
    }
  }
  console.warn(
    "Snipe Hunt Rust/WASM failed to initialize; using the deterministic preview engine.",
    wasmInitializationError,
  );
  return new FallbackEngineAdapter();
}
