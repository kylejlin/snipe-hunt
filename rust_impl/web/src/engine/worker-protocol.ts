import type { AnalysisRequest, AnalysisResult } from "./types";

export type WorkerRequest =
  | { type: "analyze"; payload: AnalysisRequest }
  | { type: "cancel"; requestId: number };

export type WorkerResponse =
  | { type: "result"; payload: AnalysisResult }
  | { type: "error"; requestId: number; message: string };
