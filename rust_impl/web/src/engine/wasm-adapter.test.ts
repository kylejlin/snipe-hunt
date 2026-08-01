import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AnalysisRequest,
  LiveAnalysisRequest,
  LiveAnalysisUpdate,
  Position,
  TurnMove,
} from "./types";
import type { WorkerRequest, WorkerResponse } from "./worker-protocol";

vi.mock("./wasm-runtime", () => ({
  wasmApplyMove: vi.fn(),
  wasmCanonicalizePosition: vi.fn(),
  wasmCreateGame: vi.fn(),
  wasmLegalMoves: vi.fn(),
  wasmPreviewFirstStep: vi.fn(),
}));

import { WasmComputerAgent, WasmLiveAnalyzer } from "./wasm-adapter";

class MockWorker {
  static instances: MockWorker[] = [];

  onmessage: ((event: MessageEvent<WorkerResponse>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  posted: WorkerRequest[] = [];
  terminated = false;

  constructor() {
    MockWorker.instances.push(this);
  }

  postMessage(message: WorkerRequest): void {
    this.posted.push(message);
  }

  terminate(): void {
    this.terminated = true;
  }

  emit(message: WorkerResponse): void {
    this.onmessage?.({ data: message } as MessageEvent<WorkerResponse>);
  }
}

const position: Position = {
  schemaVersion: 1,
  positionKey: "position",
  seed: 1,
  turn: "Alpha",
  turnNumber: 1,
  winner: null,
  leadingAction: null,
  locations: {
    "alpha-reserve": [],
    "beta-reserve": [],
    "row-1": [],
    "row-2": [],
    "row-3": [],
    "row-4": [],
    "row-5": [],
    "row-6": [],
  },
};

const move: TurnMove = {
  id: "move",
  positionKey: position.positionKey,
  player: "Alpha",
  label: "Alpha 2",
  steps: [],
  captures: { animals: [], snipe: null },
};

const request: LiveAnalysisRequest = {
  position,
  timeLimitMs: 30_000,
  requestId: 7,
  strategy: "avocado",
};

const agentRequest: AnalysisRequest = request;

const update: LiveAnalysisUpdate = {
  requestId: request.requestId,
  positionKey: position.positionKey,
  bestMove: move,
  evaluation: { kind: "estimate", millipoints: 750 },
  ticks: 3_624,
  elapsedMs: 18_662,
  recommendedLine: [move],
  strategy: "avocado",
  engineName: "Avocado",
};

const originalWorker = globalThis.Worker;

beforeEach(() => {
  MockWorker.instances = [];
  Object.defineProperty(globalThis, "Worker", {
    configurable: true,
    value: MockWorker,
  });
});

afterEach(() => {
  Object.defineProperty(globalThis, "Worker", {
    configurable: true,
    value: originalWorker,
  });
});

describe("WasmLiveAnalyzer", () => {
  it("returns the last completed update when WASM reaches its memory limit", async () => {
    const analyzer = new WasmLiveAnalyzer();
    const onProgress = vi.fn();
    const result = analyzer.analyze(
      request,
      onProgress,
      new AbortController().signal,
    );
    const worker = MockWorker.instances[0];

    worker.emit({ type: "analysis-progress", payload: update });
    worker.emit({
      type: "error",
      requestId: request.requestId,
      message: "unreachable",
      code: "memory-limit",
    });

    await expect(result).resolves.toEqual({
      ...update,
      stoppedReason: "memory-limit",
    });
    expect(onProgress).toHaveBeenCalledWith(update);
    expect(worker.terminated).toBe(true);
  });

  it("keeps ordinary worker failures as errors", async () => {
    const analyzer = new WasmLiveAnalyzer();
    const result = analyzer.analyze(
      request,
      vi.fn(),
      new AbortController().signal,
    );
    const worker = MockWorker.instances[0];

    worker.emit({
      type: "error",
      requestId: request.requestId,
      message: "generated ply is illegal",
    });

    await expect(result).rejects.toThrow("generated ply is illegal");
  });
});

describe("WasmComputerAgent", () => {
  it("plays the last completed move when WASM reaches its memory limit", async () => {
    const agent = new WasmComputerAgent();
    const result = agent.chooseMove(
      agentRequest,
      new AbortController().signal,
    );
    const worker = MockWorker.instances[0];

    worker.emit({ type: "agent-progress", payload: update });
    worker.emit({
      type: "error",
      requestId: agentRequest.requestId,
      message: "unreachable",
      code: "memory-limit",
    });

    await expect(result).resolves.toEqual(update);
    expect(worker.terminated).toBe(true);
  });

  it("keeps ordinary worker failures as errors", async () => {
    const agent = new WasmComputerAgent();
    const result = agent.chooseMove(
      agentRequest,
      new AbortController().signal,
    );
    const worker = MockWorker.instances[0];

    worker.emit({
      type: "error",
      requestId: agentRequest.requestId,
      message: "generated ply is illegal",
    });

    await expect(result).rejects.toThrow("generated ply is illegal");
  });
});
