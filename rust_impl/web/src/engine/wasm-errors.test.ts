import { describe, expect, it } from "vitest";
import { isWasmMemoryLimitTrap } from "./wasm-errors";

describe("WASM memory trap detection", () => {
  it("recognizes an unreachable trap only when linear memory is exhausted", () => {
    const trap = new WebAssembly.RuntimeError("unreachable");

    expect(isWasmMemoryLimitTrap(trap, 2.5 * 1024 ** 3)).toBe(true);
    expect(isWasmMemoryLimitTrap(trap, 512 * 1024 ** 2)).toBe(false);
    expect(
      isWasmMemoryLimitTrap(
        new WebAssembly.RuntimeError("out of bounds"),
        2.5 * 1024 ** 3,
      ),
    ).toBe(false);
    expect(
      isWasmMemoryLimitTrap(new Error("unreachable"), 2.5 * 1024 ** 3),
    ).toBe(false);
  });
});
