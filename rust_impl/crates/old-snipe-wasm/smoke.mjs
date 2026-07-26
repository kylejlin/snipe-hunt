import { readFile } from "node:fs/promises";

import init, {
  analyze,
  apply_move,
  create_game,
  legal_moves,
} from "../../web/src/wasm/pkg/old_snipe_wasm.js";

const packageDirectory = new URL("../../web/src/wasm/pkg/", import.meta.url);
const wasmBytes = await readFile(new URL("old_snipe_wasm_bg.wasm", packageDirectory));
await init({ module_or_path: wasmBytes });

const budgets = process.argv.slice(2).map(Number);
const requestedBudgets = budgets.length > 0 ? budgets : [250, 1_000, 5_000];
const position = JSON.parse(create_game(7_071));

for (const timeLimitMs of requestedBudgets) {
  if (!Number.isSafeInteger(timeLimitMs) || timeLimitMs < 0) {
    throw new Error(`invalid time budget: ${timeLimitMs}`);
  }

  const wallStart = performance.now();
  const result = JSON.parse(
    analyze(
      JSON.stringify({
        position,
        timeLimitMs,
        requestId: timeLimitMs,
      }),
    ),
  );
  const wallElapsedMs = Math.round(performance.now() - wallStart);

  console.log(
    JSON.stringify({
      requestedMs: timeLimitMs,
      reportedMs: result.elapsedMs,
      wallElapsedMs,
      depth: result.depth,
      nodes: result.nodes,
      legalBestMove: typeof result.bestMove?.id === "string",
    }),
  );
}

// Exercise the browser's history-aware request shape separately so the timing
// samples above remain directly comparable to pre-history baselines.
const firstMove = JSON.parse(legal_moves(JSON.stringify(position)))[0];
const nextPosition = JSON.parse(
  apply_move(JSON.stringify(position), JSON.stringify(firstMove)),
);
const historyResult = JSON.parse(
  analyze(
    JSON.stringify({
      position: nextPosition,
      timeLimitMs: 1,
      requestId: 0,
      history: [position],
    }),
  ),
);
console.log(
  JSON.stringify({
    historyPositions: 1,
    historyRequestAccepted: typeof historyResult.bestMove?.id === "string",
  }),
);
