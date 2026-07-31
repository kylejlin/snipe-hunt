const LARGE_WASM_MEMORY_BYTES = 2 * 1024 * 1024 * 1024;

/**
 * Rust allocation failure is exposed as a generic `unreachable` WASM trap.
 * Require a very large linear memory as corroborating evidence so ordinary
 * Rust panics retain their original error behavior.
 */
export function isWasmMemoryLimitTrap(
  reason: unknown,
  memoryBytes: number,
): boolean {
  return (
    reason instanceof WebAssembly.RuntimeError &&
    reason.message === "unreachable" &&
    memoryBytes >= LARGE_WASM_MEMORY_BYTES
  );
}
