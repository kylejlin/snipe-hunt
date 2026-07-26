import type { EngineServices, MoveStep, Position, TurnMove } from "./types";
import {
  WasmComputerAgent,
  WasmLiveAnalyzer,
  WasmRulesEngine,
} from "./wasm-adapter";
import { wasmInitializationError, wasmReady } from "./wasm-runtime";

/**
 * Builds the clean Rust engine services.
 *
 * Rust/WASM is the only rules and search implementation.
 */
export function createEngineServices(): EngineServices {
  return {
    rules: new WasmRulesEngine(),
    computerAgent: new WasmComputerAgent(),
    analyzer: new WasmLiveAnalyzer(),
  };
}

export const engineInitializationError =
  wasmReady ? null : (wasmInitializationError ?? new Error("Snipe Hunt WASM failed to initialize."));

export type { MoveStep, Position, TurnMove };
