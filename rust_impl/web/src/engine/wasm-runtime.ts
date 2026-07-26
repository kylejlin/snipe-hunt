import init, {
  apply_move,
  canonicalize_position,
  create_game,
  legal_moves,
  preview_first_step,
} from "../wasm/pkg/snipe_wasm.js";
import type { MoveStep, Position, TurnMove } from "./types";

export let wasmInitializationError: Error | null = null;

try {
  await init();
} catch (reason) {
  wasmInitializationError =
    reason instanceof Error ? reason : new Error(`WASM initialization failed: ${String(reason)}`);
}

export const wasmReady = wasmInitializationError === null;

function requireWasm(): void {
  if (wasmInitializationError) throw wasmInitializationError;
}

function decode<T>(json: string): T {
  return JSON.parse(json) as T;
}

export function wasmCreateGame(seed = 7_071): Position {
  requireWasm();
  return decode<Position>(create_game(seed >>> 0));
}

export function wasmCanonicalizePosition(position: Position): Position {
  requireWasm();
  return decode<Position>(canonicalize_position(JSON.stringify(position)));
}

export function wasmLegalMoves(position: Position): TurnMove[] {
  requireWasm();
  return decode<TurnMove[]>(legal_moves(JSON.stringify(position)));
}

export function wasmPreviewFirstStep(position: Position, step: MoveStep): Position {
  requireWasm();
  return decode<Position>(
    preview_first_step(JSON.stringify(position), JSON.stringify(step)),
  );
}

export function wasmApplyMove(position: Position, move: TurnMove): Position {
  requireWasm();
  return decode<Position>(apply_move(JSON.stringify(position), JSON.stringify(move)));
}
