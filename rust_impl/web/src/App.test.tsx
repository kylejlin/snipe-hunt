import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AnalysisResult,
  EngineServices,
  MoveStep,
  Position,
  TurnMove,
} from "./engine/types";

const { services, move, applyMove } = vi.hoisted(() => {
  const rabbit = {
    pieceKey: "alpha:animal:3",
    animal: "Rabbit",
    owner: "Alpha" as const,
    isSnipe: false,
    canRetreat: true,
  };
  const initial: Position = {
    schemaVersion: 1,
    positionKey: "position:initial",
    seed: 7,
    turn: "Alpha",
    turnNumber: 1,
    winner: null,
    leadingAction: null,
    locations: {
      "alpha-reserve": [],
      "beta-reserve": [],
      "row-1": [rabbit, { ...rabbit }],
      "row-2": [],
      "row-3": [],
      "row-4": [],
      "row-5": [],
      "row-6": [],
    },
  };
  const step: MoveStep = {
    pieceKey: rabbit.pieceKey,
    animal: rabbit.animal,
    owner: rabbit.owner,
    isSnipe: false,
    from: "row-1",
    to: "row-2",
    capture: { animals: [], snipe: null },
  };
  const move: TurnMove = {
    id: "rabbit-2",
    positionKey: initial.positionKey,
    player: "Alpha",
    label: "Rabbit 2",
    steps: [step],
    captures: { animals: [], snipe: null },
  };
  const after: Position = {
    ...initial,
    positionKey: "position:after",
    turn: "Beta",
    turnNumber: 2,
    locations: {
      ...initial.locations,
      "row-1": [rabbit],
      "row-2": [rabbit],
    },
  };
  const result = (requestId: number): AnalysisResult => ({
    requestId,
    positionKey: initial.positionKey,
    bestMove: move,
    evaluation: { kind: "estimate", millipoints: 0 },
    ticks: 1,
    elapsedMs: 1,
    recommendedLine: [move],
    strategy: "cherry",
    engineName: "test",
  });
  const applyMove = vi.fn(() => after);
  const services = {
    rules: {
      name: "test",
      createGame: () => initial,
      canonicalizePosition: (position: Position) => position,
      legalMoves: (position: Position) =>
        position.positionKey === initial.positionKey ? [move] : [],
      previewFirstStep: (position: Position) => position,
      applyMove,
    },
    computerAgent: {
      chooseMove: ({ requestId }) => Promise.resolve(result(requestId)),
      dispose: () => undefined,
    },
    analyzer: {
      analyze: ({ requestId }, onProgress) => {
        const update = result(requestId);
        onProgress(update);
        return Promise.resolve(update);
      },
      dispose: () => undefined,
    },
  } satisfies EngineServices;
  return { services, move, applyMove };
});

vi.mock("./engine/engine-services", () => ({
  createEngineServices: () => services,
  engineInitializationError: null,
}));

import App, { formatAlphaScore, formatEvaluation } from "./App";

afterEach(cleanup);

beforeEach(() => {
  localStorage.clear();
  applyMove.mockClear();
});

describe("value-semantic card interaction", () => {
  it("keeps the exact clicked occurrence highlighted while selecting the shared value", () => {
    render(<App />);
    const rabbits = screen.getAllByRole("button", {
      name: "Alpha Rabbit, retreater",
    });

    fireEvent.click(rabbits[1]);

    expect(rabbits[0]).toHaveAttribute("aria-pressed", "false");
    expect(rabbits[1]).toHaveAttribute("aria-pressed", "true");
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );
    expect(applyMove).toHaveBeenCalledWith(
      expect.objectContaining({ positionKey: "position:initial" }),
      move,
    );
  });

  it("toggles only the occurrence that was clicked", () => {
    render(<App />);
    const rabbits = screen.getAllByRole("button", {
      name: "Alpha Rabbit, retreater",
    });
    fireEvent.click(rabbits[0]);
    fireEvent.click(rabbits[0]);
    expect(rabbits[0]).toHaveAttribute("aria-pressed", "false");
    expect(rabbits[1]).toHaveAttribute("aria-pressed", "false");
  });

  it("animates from the exact interchangeable occurrence that was clicked", () => {
    const originalAnimate = Object.getOwnPropertyDescriptor(
      HTMLElement.prototype,
      "animate",
    );
    const animatedOrigins: string[] = [];
    Object.defineProperty(HTMLElement.prototype, "animate", {
      configurable: true,
      value: vi.fn(function (this: HTMLElement) {
        animatedOrigins.push(this.dataset.animationOrigin ?? "");
        return {
          finish: vi.fn(),
          finished: new Promise<void>(() => undefined),
        };
      }),
    });
    const rect = vi
      .spyOn(HTMLElement.prototype, "getBoundingClientRect")
      .mockReturnValue({
        x: 0,
        y: 0,
        top: 0,
        right: 50,
        bottom: 70,
        left: 0,
        width: 50,
        height: 70,
        toJSON: () => ({}),
      });
    try {
      render(<App />);
      const rabbits = screen.getAllByRole("button", {
        name: "Alpha Rabbit, retreater",
      });
      fireEvent.click(rabbits[0]);
      fireEvent.click(
        screen.getByRole("button", { name: "Move selected card to Rank 2" }),
      );
      expect(animatedOrigins).toContain("alpha:animal:3@row-1#0");
      expect(animatedOrigins).not.toContain("alpha:animal:3@row-1#1");
    } finally {
      rect.mockRestore();
      if (originalAnimate) {
        Object.defineProperty(
          HTMLElement.prototype,
          "animate",
          originalAnimate,
        );
      } else {
        delete (HTMLElement.prototype as { animate?: unknown }).animate;
      }
    }
  });
});

describe("presentation contract", () => {
  it("shows the package version", () => {
    render(<App />);
    expect(screen.getByText("Version 0.47.0")).toBeInTheDocument();
  });

  it("lists the remaining strategies alphabetically", () => {
    render(<App />);
    const strategy = screen.getByLabelText("Strategy") as HTMLSelectElement;
    expect(Array.from(strategy.options, ({ text }) => text)).toEqual([
      "Avocado",
      "Cherry",
      "Fajita",
    ]);
  });

  it("formats Alpha evaluations", () => {
    expect(formatAlphaScore(125, "Alpha")).toBe("+1.3");
    expect(formatAlphaScore(999_997, "Alpha")).toBe("+#3");
    expect(formatAlphaScore(999_998, "Beta")).toBe("-#2");
  });

  it("formats integer engine evaluations as point scores", () => {
    expect(
      formatEvaluation({ kind: "estimate", millipoints: 1_250 }),
    ).toBe("+1.3");
    expect(
      formatEvaluation({ kind: "estimate", millipoints: -100_000 }),
    ).toBe("-100.0");
    expect(
      formatEvaluation({ kind: "mate", winner: "Beta", plies: 7 }),
    ).toBe("-#7");
  });
});

describe("overlays", () => {
  it("closes Game Log settings on outside interaction and Escape", () => {
    render(<App />);
    const settings = screen.getByRole("button", {
      name: "Game Log settings",
    });

    fireEvent.click(settings);
    expect(screen.getByText("Export")).toBeInTheDocument();
    fireEvent.pointerDown(document.body);
    expect(screen.queryByText("Export")).not.toBeInTheDocument();

    fireEvent.click(settings);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByText("Export")).not.toBeInTheDocument();
    expect(settings).toHaveFocus();
  });

  it("commits analysis time before an outside interaction closes settings", () => {
    render(<App />);
    const settings = screen.getByRole("button", {
      name: "Game Log settings",
    });

    fireEvent.click(settings);
    const analysisTime = screen.getByRole("textbox", {
      name: "Analysis time",
    });
    analysisTime.focus();
    fireEvent.change(analysisTime, { target: { value: "7" } });
    fireEvent.pointerDown(document.body);

    fireEvent.click(settings);
    expect(
      screen.getByRole("textbox", { name: "Analysis time" }),
    ).toHaveValue("7");
  });

  it("uses an in-page confirmation before resetting the game", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Reset game" }));
    const dialog = screen.getByRole("alertdialog");
    expect(dialog).toHaveTextContent("Start a fresh game?");
    expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus();

    fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });
});
