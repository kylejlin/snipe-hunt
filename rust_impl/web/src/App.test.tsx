import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  EngineAdapter,
  Location,
  Position,
  TurnMove,
} from "./engine/types";

const { engine, earlier, current, move, earlierMove } = vi.hoisted(() => {
  const locations = (ratLocation: Location): Position["locations"] => ({
    "alpha-reserve": [],
    "beta-reserve": [],
    "row-1":
      ratLocation === "row-1"
        ? [
            {
              id: "animal-0",
              animal: "Rat",
              owner: "Alpha",
              isSnipe: false,
              canRetreat: true,
            },
          ]
        : [],
    "row-2": [
      ...(ratLocation === "row-2"
        ? [
            {
              id: "animal-0",
              animal: "Rat",
              owner: "Alpha" as const,
              isSnipe: false,
              canRetreat: true,
            },
          ]
        : []),
      {
        id: "animal-1",
        animal: "Ox",
        owner: "Alpha" as const,
        isSnipe: false,
        canRetreat: false,
      },
    ],
    "row-3": [],
    "row-4": [],
    "row-5": [],
    "row-6": [],
  });
  const earlier: Position = {
    schemaVersion: 1,
    seed: 1,
    turn: "Alpha",
    turnNumber: 1,
    winner: null,
    locations: locations("row-1"),
  };
  const current: Position = {
    ...earlier,
    seed: 2,
    turnNumber: 2,
  };
  const move: TurnMove = {
    id: "two-step",
    player: "Alpha",
    label: "Rat 2, Ox 3",
    steps: [
      { cardId: "animal-0", from: "row-1", to: "row-2" },
      { cardId: "animal-1", from: "row-2", to: "row-3" },
    ],
    captures: [],
  };
  const earlierMove: TurnMove = {
    id: "earlier-two-step",
    player: "Alpha",
    label: "Ox 3, Rat 2",
    steps: [
      { cardId: "animal-1", from: "row-2", to: "row-3" },
      { cardId: "animal-0", from: "row-1", to: "row-2" },
    ],
    captures: [],
  };
  const adapter = {
    name: "test engine",
    createGame: () => current,
    legalMoves: (position: Position) =>
      position.seed === 2 ? [move] : [earlierMove],
    previewFirstStep: (position: Position, step: TurnMove["steps"][number]) => {
      const expectedCard = position.seed === 2 ? "animal-0" : "animal-1";
      if (step.cardId !== expectedCard) {
        throw new Error("first animal step is not legal in this position");
      }
      if (position.seed === 2) {
        return { ...position, locations: locations("row-2") };
      }
      return {
        ...position,
        locations: {
          ...position.locations,
          "row-2": position.locations["row-2"].filter(
            (card) => card.id !== "animal-1",
          ),
          "row-3": [position.locations["row-2"].find(
            (card) => card.id === "animal-1",
          )!],
        },
      };
    },
    applyMove: () => earlier,
    analyze: () => Promise.reject(new Error("analysis should remain off")),
    dispose: () => undefined,
  } satisfies EngineAdapter;
  return { engine: adapter, earlier, current, move, earlierMove };
});

vi.mock("./engine/fallback-adapter", () => ({
  createEngineAdapter: () => engine,
}));

import App from "./App";

describe("pending subply history navigation", () => {
  afterEach(cleanup);

  beforeEach(() => {
    localStorage.clear();
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 1,
        timeline: [
          { position: earlier, move: null },
          { position: current, move },
        ],
        cursor: 1,
        mode: "manual",
        manualAnalysis: false,
        timeLimitSeconds: 5,
      }),
    );
  });

  it("discards a pending first subply before moving backward in history", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha Rat, retreater" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );
    expect(
      screen.getByRole("button", { name: "Undo first subply" }),
    ).toBeInTheDocument();
    const pendingPly = screen.getByLabelText("Pending ply");
    expect(pendingPly).toHaveTextContent("1α.");
    expect(pendingPly).toHaveTextContent("Rat 2, ...");

    fireEvent.click(screen.getByRole("button", { name: "← Back" }));

    expect(
      screen.queryByRole("button", { name: "Undo first subply" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Pending ply")).not.toBeInTheDocument();
    expect(screen.getByText("1 / 2")).toBeInTheDocument();
  });

  it("discards a pending first subply before moving forward in history", () => {
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 1,
        timeline: [
          { position: earlier, move: null },
          { position: current, move: earlierMove },
        ],
        cursor: 0,
        mode: "manual",
        manualAnalysis: false,
        timeLimitSeconds: 5,
      }),
    );
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha Ox" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 3" }),
    );
    expect(
      screen.getByRole("button", { name: "Undo first subply" }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Forward →" }));

    expect(
      screen.queryByRole("button", { name: "Undo first subply" }),
    ).not.toBeInTheDocument();
    expect(screen.getByText("2 / 2")).toBeInTheDocument();
  });
});
