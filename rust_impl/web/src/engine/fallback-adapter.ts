import {
  analyzeFallback,
  analyzeFallbackAtDepth,
  applyFallbackMove,
  createFallbackGame,
  fallbackLegalMoves,
  previewFallbackFirstStep,
} from "./fallback-core";
import type { WorkerResponse } from "./worker-protocol";
import type {
  AnalysisRequest,
  AnalysisResult,
  ComputerAgent,
  EngineServices,
  LiveAnalysisRequest,
  LiveAnalysisUpdate,
  LiveAnalyzer,
  MoveStep,
  Position,
  RulesEngine,
  TurnMove,
} from "./types";
import {
  WasmComputerAgent,
  WasmLiveAnalyzer,
  WasmRulesEngine,
} from "./wasm-adapter";
import { wasmInitializationError, wasmReady } from "./wasm-runtime";

class FallbackRulesEngine implements RulesEngine {
  readonly name = "Deterministic preview rules";
  createGame = createFallbackGame;
  legalMoves = fallbackLegalMoves;
  previewFirstStep = previewFallbackFirstStep;
  applyMove = applyFallbackMove;
}

class FallbackComputerAgent implements ComputerAgent {
  private client = new FallbackWorkerClient();

  chooseMove(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult> {
    if (typeof Worker === "undefined") {
      return abortableTimeout(
        signal,
        120,
        () => analyzeFallback(request.position, request.requestId, 120),
      );
    }
    return this.client.runAgent(request, signal);
  }

  dispose(): void {
    this.client.dispose();
  }
}

class FallbackLiveAnalyzer implements LiveAnalyzer {
  private client = new FallbackWorkerClient();

  analyze(
    request: LiveAnalysisRequest,
    onProgress: (update: LiveAnalysisUpdate) => void,
    signal: AbortSignal,
  ): Promise<LiveAnalysisUpdate> {
    if (typeof Worker === "undefined") {
      return new Promise((resolve, reject) => {
        let depth = 1;
        const timer = window.setInterval(() => {
          if (signal.aborted) return;
          const update = analyzeFallbackAtDepth(
            request.position,
            request.requestId,
            depth,
            request.firstStep,
          );
          if (depth >= request.maxDepth) {
            clearInterval(timer);
            signal.removeEventListener("abort", abort);
            resolve(update);
          } else {
            onProgress(update);
            depth += 1;
          }
        }, 45);
        const abort = () => {
          clearInterval(timer);
          reject(new DOMException("Analysis cancelled.", "AbortError"));
        };
        signal.addEventListener("abort", abort, { once: true });
      });
    }
    return this.client.runAnalysis(request, onProgress, signal);
  }

  dispose(): void {
    this.client.dispose();
  }
}

class FallbackWorkerClient {
  private worker: Worker | null = null;
  private pending:
    | {
        requestId: number;
        onProgress?: (update: LiveAnalysisUpdate) => void;
        resolve: (result: AnalysisResult | LiveAnalysisUpdate) => void;
        reject: (reason: Error) => void;
        removeAbortListener: () => void;
      }
    | null = null;

  runAgent(request: AnalysisRequest, signal: AbortSignal): Promise<AnalysisResult> {
    return this.run<AnalysisResult>(
      { type: "agent", payload: request },
      request.requestId,
      undefined,
      signal,
    );
  }

  runAnalysis(
    request: LiveAnalysisRequest,
    onProgress: (update: LiveAnalysisUpdate) => void,
    signal: AbortSignal,
  ): Promise<LiveAnalysisUpdate> {
    return this.run<LiveAnalysisUpdate>(
      { type: "analysis", payload: request },
      request.requestId,
      onProgress,
      signal,
    );
  }

  dispose(): void {
    this.worker?.terminate();
    this.worker = null;
    this.finish(new Error("Fallback search disposed."));
  }

  private run<T extends AnalysisResult | LiveAnalysisUpdate>(
    message: { type: "agent"; payload: AnalysisRequest } | {
      type: "analysis";
      payload: LiveAnalysisRequest;
    },
    requestId: number,
    onProgress: ((update: LiveAnalysisUpdate) => void) | undefined,
    signal: AbortSignal,
  ): Promise<T> {
    if (signal.aborted) return Promise.reject(new DOMException("Search cancelled.", "AbortError"));
    this.ensureWorker();
    return new Promise((resolve, reject) => {
      const abort = () => {
        if (this.pending?.requestId === requestId) this.pending = null;
        reject(new DOMException("Search cancelled.", "AbortError"));
        this.worker?.postMessage({ type: "cancel", requestId });
      };
      signal.addEventListener("abort", abort, { once: true });
      this.pending = {
        requestId,
        onProgress,
        resolve: resolve as (result: AnalysisResult | LiveAnalysisUpdate) => void,
        reject,
        removeAbortListener: () => signal.removeEventListener("abort", abort),
      };
      this.worker?.postMessage(message);
    });
  }

  private ensureWorker(): void {
    if (this.worker) return;
    const worker = new Worker(new URL("./fallback.worker.ts", import.meta.url), {
      type: "module",
    });
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
      const message = event.data;
      const requestId =
        message.type === "error" ? message.requestId : message.payload.requestId;
      if (!this.pending || this.pending.requestId !== requestId) return;
      if (message.type === "analysis-progress") {
        this.pending.onProgress?.(message.payload);
      } else if (message.type === "error") {
        this.finish(new Error(message.message));
      } else {
        this.finish(undefined, message.payload);
      }
    };
    worker.onerror = (event) => {
      this.finish(new Error(event.message || "Fallback search worker failed."));
      worker.terminate();
      if (this.worker === worker) this.worker = null;
    };
    this.worker = worker;
  }

  private finish(
    error?: Error,
    result?: AnalysisResult | LiveAnalysisUpdate,
  ): void {
    const pending = this.pending;
    this.pending = null;
    pending?.removeAbortListener();
    if (!pending) return;
    if (error) pending.reject(error);
    else if (result) pending.resolve(result);
  }
}

function abortableTimeout<T>(
  signal: AbortSignal,
  delay: number,
  result: () => T,
): Promise<T> {
  return new Promise((resolve, reject) => {
    const timer = window.setTimeout(() => resolve(result()), delay);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        reject(new DOMException("Search cancelled.", "AbortError"));
      },
      { once: true },
    );
  });
}

export function createEngineServices(): EngineServices {
  if (wasmReady) {
    try {
      return {
        rules: new WasmRulesEngine(),
        computerAgent: new WasmComputerAgent(),
        analyzer: new WasmLiveAnalyzer(),
      };
    } catch (reason) {
      console.warn(
        "Snipe Hunt Rust/WASM workers failed to initialize; using deterministic preview search.",
        reason,
      );
    }
  } else {
    console.warn(
      "Snipe Hunt Rust/WASM failed to initialize; using deterministic preview search.",
      wasmInitializationError,
    );
  }
  return {
    rules: new FallbackRulesEngine(),
    computerAgent: new FallbackComputerAgent(),
    analyzer: new FallbackLiveAnalyzer(),
  };
}

export type { MoveStep, Position, TurnMove };
