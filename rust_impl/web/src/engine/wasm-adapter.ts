import type { WorkerResponse } from "./worker-protocol";
import type {
  AnalysisRequest,
  AnalysisResult,
  ComputerAgent,
  LiveAnalysisRequest,
  LiveAnalysisUpdate,
  LiveAnalyzer,
  MoveStep,
  Position,
  RulesEngine,
  TurnMove,
} from "./types";
import {
  wasmApplyMove,
  wasmCanonicalizePosition,
  wasmCreateGame,
  wasmLegalMoves,
  wasmPreviewFirstStep,
} from "./wasm-runtime";

export class WasmRulesEngine implements RulesEngine {
  readonly name = "Snipe Hunt Rust rules";

  createGame(seed?: number): Position {
    return wasmCreateGame(seed);
  }

  canonicalizePosition(position: Position): Position {
    return wasmCanonicalizePosition(position);
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
}

export class WasmComputerAgent implements ComputerAgent {
  private worker: Worker | null = null;
  private pending:
    | {
        requestId: number;
        positionKey: string;
        resolve: (result: AnalysisResult) => void;
        reject: (reason: Error) => void;
        removeAbortListener: () => void;
      }
    | null = null;
  private disposed = false;

  chooseMove(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult> {
    if (this.disposed) return Promise.reject(new Error("Computer agent disposed."));
    if (signal.aborted) return Promise.reject(abortError("Computer move cancelled."));
    this.rejectPending(abortError("Computer move superseded."));
    this.restartWorker();
    this.ensureWorker();
    return new Promise((resolve, reject) => {
      const abort = () => {
        if (this.pending?.requestId === request.requestId) this.pending = null;
        reject(abortError("Computer move cancelled."));
        this.restartWorker();
      };
      signal.addEventListener("abort", abort, { once: true });
      this.pending = {
        requestId: request.requestId,
        positionKey: request.position.positionKey,
        resolve,
        reject,
        removeAbortListener: () => signal.removeEventListener("abort", abort),
      };
      this.worker?.postMessage({ type: "agent", payload: request });
    });
  }

  dispose(): void {
    this.disposed = true;
    this.worker?.terminate();
    this.worker = null;
    this.rejectPending(new Error("Computer agent disposed."));
  }

  private ensureWorker(): void {
    if (this.worker || this.disposed) return;
    const worker = new Worker(new URL("./wasm.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;
      const requestId =
        message.type === "error" ? message.requestId : message.payload.requestId;
      if (!this.pending || this.pending.requestId !== requestId) return;
      if (message.type === "agent-result") {
        if (message.payload.positionKey !== this.pending.positionKey) {
          this.rejectPending(new Error("Computer returned a stale position."));
          return;
        }
        const pending = this.takePending();
        pending?.resolve(message.payload);
      } else if (message.type === "error") {
        const pending = this.takePending();
        pending?.reject(new Error(message.message));
      }
    };
    worker.onerror = (event) => {
      this.rejectPending(new Error(event.message || "Computer agent worker failed."));
      worker.terminate();
      if (this.worker === worker) this.worker = null;
    };
    this.worker = worker;
  }

  private restartWorker(): void {
    this.worker?.terminate();
    this.worker = null;
    if (!this.disposed) this.ensureWorker();
  }

  private takePending() {
    const pending = this.pending;
    this.pending = null;
    pending?.removeAbortListener();
    return pending;
  }

  private rejectPending(reason: Error): void {
    this.takePending()?.reject(reason);
  }
}

export class WasmLiveAnalyzer implements LiveAnalyzer {
  private worker: Worker | null = null;
  private pending:
    | {
        requestId: number;
        positionKey: string;
        lastProgress: LiveAnalysisUpdate | null;
        onProgress: (update: LiveAnalysisUpdate) => void;
        resolve: (result: LiveAnalysisUpdate) => void;
        reject: (reason: Error) => void;
        removeAbortListener: () => void;
      }
    | null = null;
  private disposed = false;

  analyze(
    request: LiveAnalysisRequest,
    onProgress: (update: LiveAnalysisUpdate) => void,
    signal: AbortSignal,
  ): Promise<LiveAnalysisUpdate> {
    if (this.disposed) return Promise.reject(new Error("Analyzer disposed."));
    if (signal.aborted) return Promise.reject(abortError("Analysis cancelled."));
    this.rejectPending(abortError("Analysis superseded."));
    this.restartWorker();
    this.ensureWorker();
    return new Promise((resolve, reject) => {
      const abort = () => {
        if (this.pending?.requestId === request.requestId) this.pending = null;
        reject(abortError("Analysis cancelled."));
        this.restartWorker();
      };
      signal.addEventListener("abort", abort, { once: true });
      this.pending = {
        requestId: request.requestId,
        positionKey: request.position.positionKey,
        lastProgress: null,
        onProgress,
        resolve,
        reject,
        removeAbortListener: () => signal.removeEventListener("abort", abort),
      };
      this.worker?.postMessage({ type: "analysis", payload: request });
    });
  }

  dispose(): void {
    this.disposed = true;
    this.worker?.terminate();
    this.worker = null;
    this.rejectPending(new Error("Analyzer disposed."));
  }

  private ensureWorker(): void {
    if (this.worker || this.disposed) return;
    const worker = new Worker(new URL("./wasm.worker.ts", import.meta.url), { type: "module" });
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;
      const requestId =
        message.type === "error" ? message.requestId : message.payload.requestId;
      if (!this.pending || this.pending.requestId !== requestId) return;
      if (message.type === "analysis-progress") {
        if (message.payload.positionKey !== this.pending.positionKey) return;
        this.pending.lastProgress = message.payload;
        this.pending.onProgress(message.payload);
      } else if (message.type === "analysis-complete") {
        if (message.payload.positionKey !== this.pending.positionKey) {
          this.rejectPending(new Error("Analysis returned a stale position."));
          return;
        }
        const pending = this.takePending();
        pending?.resolve(message.payload);
      } else if (message.type === "error") {
        const pending = this.takePending();
        if (message.code === "memory-limit") {
          worker.terminate();
          if (this.worker === worker) this.worker = null;
          if (pending?.lastProgress) {
            pending.resolve({
              ...pending.lastProgress,
              stoppedReason: "memory-limit",
            });
          } else {
            pending?.reject(
              new Error(
                "Analysis reached the browser memory ceiling before completing a result.",
              ),
            );
          }
        } else {
          pending?.reject(new Error(message.message));
        }
      }
    };
    worker.onerror = (event) => {
      this.rejectPending(new Error(event.message || "Analysis worker failed."));
      worker.terminate();
      if (this.worker === worker) this.worker = null;
    };
    this.worker = worker;
  }

  private restartWorker(): void {
    this.worker?.terminate();
    this.worker = null;
    if (!this.disposed) this.ensureWorker();
  }

  private takePending() {
    const pending = this.pending;
    this.pending = null;
    pending?.removeAbortListener();
    return pending;
  }

  private rejectPending(reason: Error): void {
    this.takePending()?.reject(reason);
  }
}

function abortError(message: string): DOMException {
  return new DOMException(message, "AbortError");
}
