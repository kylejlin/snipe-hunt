import type {
  AnalysisRequest,
  AnalysisResult,
  LiveAnalysisRequest,
  LiveAnalysisUpdate,
} from "./types";

export type WorkerRequest =
  | { type: "agent"; payload: AnalysisRequest }
  | { type: "analysis"; payload: LiveAnalysisRequest }
  | { type: "cancel"; requestId: number };

export type WorkerResponse =
  | { type: "agent-result"; payload: AnalysisResult }
  | { type: "analysis-progress"; payload: LiveAnalysisUpdate }
  | { type: "analysis-complete"; payload: LiveAnalysisUpdate }
  | {
      type: "error";
      requestId: number;
      message: string;
      code?: "memory-limit";
    };
