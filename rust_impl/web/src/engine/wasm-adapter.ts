import type { WorkerResponse } from "./worker-protocol";
import type {
  AnalysisRequest,
  AnalysisResult,
  EngineAdapter,
  MoveStep,
  Position,
  TurnMove,
} from "./types";
import {
  wasmApplyMove,
  wasmCreateGame,
  wasmLegalMoves,
  wasmPreviewFirstStep,
} from "./wasm-runtime";

interface PendingAnalysis {
  resolve: (result: AnalysisResult) => void;
  reject: (reason: Error) => void;
  removeAbortListener: () => void;
}

export class WasmEngineAdapter implements EngineAdapter {
  readonly name = "Snipe Hunt Rust alpha-beta";
  private worker: Worker | null = null;
  private pending = new Map<number, PendingAnalysis>();
  private disposed = false;

  constructor() {
    this.spawnWorker();
  }

  createGame(seed?: number): Position {
    return wasmCreateGame(seed);
  }

  legalMoves(position: Position): TurnMove[] {
    return wasmLegalMoves(position);
  }

  previewFirstStep(position: Position, step: MoveStep): Position {
    return wasmPreviewFirstStep(position, step);
  }

  applyMove(position: Position, move: TurnMove): Position {
    return wasmApplyMove(position, move);
  }

  analyze(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult> {
    if (this.disposed) return Promise.reject(new Error("Engine disposed."));
    if (signal.aborted) return Promise.reject(new DOMException("Analysis cancelled.", "AbortError"));
    if (!this.worker) this.spawnWorker();

    return new Promise((resolve, reject) => {
      const abort = () => {
        this.pending.delete(request.requestId);
        reject(new DOMException("Analysis cancelled.", "AbortError"));
        // A synchronous WASM search cannot consume a cancel event. Termination
        // stops it immediately; a fresh worker is ready for the next request.
        this.restartWorker(request.requestId);
      };
      signal.addEventListener("abort", abort, { once: true });
      this.pending.set(request.requestId, {
        resolve,
        reject,
        removeAbortListener: () => signal.removeEventListener("abort", abort),
      });
      this.worker?.postMessage({ type: "analyze", payload: request });
    });
  }

  dispose(): void {
    this.disposed = true;
    this.worker?.terminate();
    this.worker = null;
    this.rejectAll(new Error("Engine disposed."));
  }

  private spawnWorker(): void {
    if (typeof Worker === "undefined" || this.disposed) {
      throw new Error("Web Workers are required by the Rust search engine.");
    }
    const worker = new Worker(new URL("./wasm.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;
      const requestId = message.type === "result" ? message.payload.requestId : message.requestId;
      const pending = this.pending.get(requestId);
      // A response from an aborted/stale request is intentionally ignored.
      if (!pending) return;
      this.pending.delete(requestId);
      pending.removeAbortListener();
      if (message.type === "result") pending.resolve(message.payload);
      else pending.reject(new Error(message.message));
    };
    worker.onerror = (event) => {
      this.rejectAll(new Error(event.message || "Rust search worker failed."));
      worker.terminate();
      if (this.worker === worker) this.worker = null;
    };
    this.worker = worker;
  }

  private restartWorker(cancelledRequestId: number): void {
    this.worker?.terminate();
    this.worker = null;
    for (const [requestId, pending] of this.pending) {
      if (requestId === cancelledRequestId) continue;
      pending.removeAbortListener();
      pending.reject(new DOMException("Analysis superseded.", "AbortError"));
      this.pending.delete(requestId);
    }
    if (!this.disposed) this.spawnWorker();
  }

  private rejectAll(reason: Error): void {
    for (const pending of this.pending.values()) {
      pending.removeAbortListener();
      pending.reject(reason);
    }
    this.pending.clear();
  }
}
