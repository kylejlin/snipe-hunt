// @ts-expect-error Vitest supplies Node built-ins; the browser bundle does not ship Node types.
import { readFileSync } from "node:fs";
// @ts-expect-error Vitest supplies Node built-ins; the browser bundle does not ship Node types.
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import type { MoveStep, Position, TurnMove } from "./engine/types";
import { parseHistory, serializeHistory } from "./history-format";
import {
  apply_move,
  canonicalize_position,
  initSync,
  legal_moves,
  preview_first_step,
} from "./wasm/pkg/snipe_wasm.js";

initSync({ module: readFileSync(resolve("src/wasm/pkg/snipe_wasm_bg.wasm")) });

const engine = {
  canonicalizePosition(position: Position): Position {
    return JSON.parse(canonicalize_position(JSON.stringify(position))) as Position;
  },
  legalMoves(position: Position): TurnMove[] {
    return JSON.parse(legal_moves(JSON.stringify(position))) as TurnMove[];
  },
  previewFirstStep(position: Position, step: MoveStep): Position {
    return JSON.parse(
      preview_first_step(JSON.stringify(position), JSON.stringify(step)),
    ) as Position;
  },
  applyMove(position: Position, move: TurnMove): Position {
    return JSON.parse(
      apply_move(JSON.stringify(position), JSON.stringify(move)),
    ) as Position;
  },
};

describe("native arena history recording", () => {
  it("round-trips through the authoritative SHGH importer", () => {
    const source = readFileSync(resolve("../../game4.shgh"), "utf8");
    const timeline = parseHistory(source, engine);
    expect(timeline).toHaveLength(27);
    expect(timeline.at(-1)?.position.winner).toBe("Alpha");
  });

  it("imports and re-exports a terminal win with its result marker", () => {
    const source = readFileSync(resolve("../../game6.shgh"), "utf8");
    const timeline = parseHistory(source, engine);

    expect(timeline).toHaveLength(23);
    expect(timeline.at(-1)?.position.winner).toBe("Alpha");
    expect(timeline.at(-1)?.move?.captures.snipe).toBe("Beta");
    const exported = serializeHistory(timeline);
    expect(exported).toMatch(/22a\. Elephant 6x\+#0\n$/);
    expect(parseHistory(exported, engine).at(-1)?.position.winner).toBe(
      "Alpha",
    );
  });
});
