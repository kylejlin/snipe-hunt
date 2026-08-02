import {
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AnalysisResult,
  EngineServices,
  MoveStep,
  Position,
  TurnMove,
} from "./engine/types";

const {
  services,
  initial,
  after,
  step,
  move,
  twoStepMove,
  applyMove,
  analysisRun,
  result,
} = vi.hoisted(() => {
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
  const twoStepMove: TurnMove = {
    ...move,
    id: "rabbit-pair",
    label: "Rabbit 2, Rabbit 2",
    steps: [
      step,
      {
        ...step,
        pieceKey: "alpha:animal:3:second",
      },
    ],
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
  const analysisRun = vi.fn(({ requestId }, onProgress) => {
    const update = result(requestId);
    onProgress(update);
    return Promise.resolve(update);
  });
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
      analyze: analysisRun,
      dispose: () => undefined,
    },
  } satisfies EngineServices;
  return {
    services,
    initial,
    after,
    step,
    move,
    twoStepMove,
    applyMove,
    analysisRun,
    result,
  };
});

vi.mock("./engine/engine-services", () => ({
  createEngineServices: () => services,
  engineInitializationError: null,
}));

import App, { formatAlphaScore, formatEvaluation } from "./App";
import { gameReducer, newGame } from "./state/game-state";
import { saveGame, STORAGE_KEY } from "./state/persistence";

afterEach(cleanup);

beforeEach(() => {
  localStorage.clear();
  applyMove.mockClear();
  analysisRun.mockReset();
  analysisRun.mockImplementation(({ requestId }, onProgress) => {
    const update = result(requestId);
    onProgress(update);
    return Promise.resolve(update);
  });
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
    expect(screen.getByText("Version 0.79.0")).toBeInTheDocument();
  });

  it("does not show a turn-status pill", () => {
    render(<App />);
    expect(screen.queryByText("Alpha to move")).not.toBeInTheDocument();
  });

  it("shows a persistent scrollbar exactly when the Game Log overflows", () => {
    render(<App />);
    const timeline = screen.getByRole("list", { name: "Game timeline" });
    const timelineWrapper = timeline.parentElement;
    expect(timelineWrapper?.querySelector(".move-list-scrollbar")).toBeNull();

    Object.defineProperties(timeline, {
      clientHeight: { configurable: true, value: 100 },
      scrollHeight: { configurable: true, value: 400 },
      scrollTop: { configurable: true, value: 100, writable: true },
    });
    fireEvent(window, new Event("resize"));

    const scrollbar = timelineWrapper?.querySelector(
      ".move-list-scrollbar",
    );
    const thumb = scrollbar?.querySelector<HTMLElement>(
      ".move-list-scrollbar__thumb",
    );
    expect(scrollbar).toBeInTheDocument();
    expect(thumb).toHaveStyle({ height: "28px", transform: "translateY(24px)" });

    Object.defineProperty(timeline, "scrollHeight", {
      configurable: true,
      value: 100,
    });
    fireEvent(window, new Event("resize"));
    expect(timelineWrapper?.querySelector(".move-list-scrollbar")).toBeNull();
  });

  it("lays out the Game Log navigation outside the scrolling timeline", () => {
    render(<App />);

    const gameLog = screen.getByRole("region", { name: "Game log" });
    const currentPosition = within(gameLog).getByLabelText("Current position");
    const timeline = within(gameLog).getByRole("list", {
      name: "Game timeline",
    });
    const navigation = within(gameLog).getByLabelText("History navigation");
    const buttonRow = navigation.querySelector(".history-navigation-buttons");

    expect(
      within(gameLog).getByRole("heading", { name: "Game Log" }),
    ).toBeInTheDocument();
    expect(currentPosition).toHaveTextContent("Initial position");
    expect(currentPosition).toHaveClass("visually-hidden");
    expect(buttonRow).toContainElement(
      within(gameLog).getByRole("button", { name: "Back" }),
    );
    expect(buttonRow).toContainElement(
      within(gameLog).getByRole("button", { name: "Forward" }),
    );
    expect(navigation.querySelector(".history-navigation-status")).toBeNull();
    expect(within(gameLog).queryByText("0/0")).not.toBeInTheDocument();
    expect(
      timeline.compareDocumentPosition(navigation) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.queryByText(/\d+ (?:ply|plies)$/)).not.toBeInTheDocument();
    expect(document.querySelector(".table-panel__topline")).toBeNull();
  });

  it("uses the requested board and line labels", () => {
    render(<App />);

    expect(screen.getAllByText("Empty rank").length).toBeGreaterThan(0);
    expect(screen.queryByText("Open field")).not.toBeInTheDocument();
  });

  it("plays suggested lines through an individual action", async () => {
    analysisRun.mockImplementation(({ requestId }, onProgress) => {
      const update = {
        ...result(requestId),
        bestMove: twoStepMove,
        recommendedLine: [twoStepMove],
      };
      onProgress(update);
      return Promise.resolve(update);
    });
    render(<App />);

    fireEvent.click(screen.getByRole("switch", { name: "Analysis" }));

    const firstAction = await screen.findByRole("button", {
      name: "Play suggested line through 1α. Rabbit 2",
    });
    expect(firstAction).toHaveClass("move-list__action");
    expect(
      screen.getByRole("button", {
        name: "Play suggested line through 1α. Rabbit 2, Rabbit 2",
      }),
    ).toBeInTheDocument();

    fireEvent.click(firstAction);

    expect(screen.getByLabelText("Current position")).toHaveTextContent(
      "1α. Rabbit 2, …",
    );
    const partialSuggestion = screen.getByRole("list", {
      name: "Suggested Line",
    });
    expect(partialSuggestion).toHaveTextContent("1α. ..., Rabbit 2");
    expect(
      within(partialSuggestion).queryByRole("button", {
        name: "Play suggested line through 1α. Rabbit 2",
      }),
    ).not.toBeInTheDocument();

    fireEvent.click(
      await screen.findByRole("button", {
        name: "Play suggested line through 1α. ..., Rabbit 2",
      }),
    );

    expect(screen.getByLabelText("Current position")).toHaveTextContent(
      "1α. Rabbit 2, Rabbit 2",
    );
  });

  it("autoscrolls the current Game Log action after moves and navigation", () => {
    const originalScrollTo = Object.getOwnPropertyDescriptor(
      HTMLElement.prototype,
      "scrollTo",
    );
    const scrollTo = vi.fn();
    Object.defineProperty(HTMLElement.prototype, "scrollTo", {
      configurable: true,
      value: scrollTo,
    });
    const rect = vi
      .spyOn(HTMLElement.prototype, "getBoundingClientRect")
      .mockImplementation(function (this: HTMLElement) {
        const top = this.classList.contains("move-list")
          ? 0
          : this.hasAttribute("aria-current")
            ? 125
            : 10;
        const height = this.classList.contains("move-list") ? 100 : 20;
        return {
          x: 0,
          y: top,
          top,
          right: 100,
          bottom: top + height,
          left: 0,
          width: 100,
          height,
          toJSON: () => ({}),
        };
      });

    try {
      render(<App />);
      expect(scrollTo).toHaveBeenCalledTimes(1);

      fireEvent.click(
        screen.getAllByRole("button", {
          name: "Alpha Rabbit, retreater",
        })[0],
      );
      fireEvent.click(
        screen.getByRole("button", { name: "Move selected card to Rank 2" }),
      );
      expect(scrollTo).toHaveBeenCalledTimes(2);

      fireEvent.click(screen.getByRole("button", { name: "Back" }));
      expect(scrollTo).toHaveBeenCalledTimes(3);

      fireEvent.click(screen.getByRole("button", { name: "Forward" }));
      expect(scrollTo).toHaveBeenCalledTimes(4);
      expect(scrollTo).toHaveBeenLastCalledWith({
        top: 45,
        behavior: "smooth",
      });
    } finally {
      rect.mockRestore();
      if (originalScrollTo) {
        Object.defineProperty(
          HTMLElement.prototype,
          "scrollTo",
          originalScrollTo,
        );
      } else {
        delete (HTMLElement.prototype as { scrollTo?: unknown }).scrollTo;
      }
    }
  });

  it("labels the initial position while keeping its zero-ply notation", () => {
    render(<App />);

    expect(screen.getByLabelText("Current position")).toHaveTextContent(
      "Initial position",
    );
    const initialPosition = screen.getByRole("button", {
      name: "Go to initial position",
    });
    expect(initialPosition).toHaveAttribute("aria-current", "true");
    expect(initialPosition).toHaveTextContent("0β.");
    expect(initialPosition).toHaveTextContent("0α.");
  });

  it("selects individual actions inside one two-action ply", () => {
    const completed = gameReducer(
      { ...newGame(initial), gameMode: "pass-and-play" },
      {
        type: "commit",
        basePositionKey: initial.positionKey,
        position: after,
        move: twoStepMove,
      },
    );
    localStorage.setItem(STORAGE_KEY, saveGame(completed));
    render(<App />);

    const currentPosition = screen.getByLabelText("Current position");
    expect(currentPosition).toHaveTextContent(
      "1α. Rabbit 2, Rabbit 2",
    );
    const firstAction = screen.getByRole("button", {
      name: "Go to position after 1α. Rabbit 2",
    });
    const secondAction = screen.getByRole("button", {
      name: "Go to position after 1α. Rabbit 2, Rabbit 2",
    });
    expect(secondAction).toHaveAttribute("aria-current", "step");

    fireEvent.click(firstAction);

    expect(currentPosition).toHaveTextContent("1α. Rabbit 2, …");
    expect(firstAction).toHaveAttribute("aria-current", "step");
    expect(secondAction).not.toHaveAttribute("aria-current");
    expect(document.body).not.toHaveTextContent(".5α");

    fireEvent.click(secondAction);

    expect(currentPosition).toHaveTextContent(
      "1α. Rabbit 2, Rabbit 2",
    );
  });

  it("renders a live first step as an incomplete ply without half notation", () => {
    const draft = gameReducer(
      { ...newGame(initial), gameMode: "pass-and-play" },
      { type: "draft", step },
    );
    localStorage.setItem(STORAGE_KEY, saveGame(draft));
    render(<App />);

    expect(screen.getByLabelText("Current position")).toHaveTextContent(
      "1α. Rabbit 2, …",
    );
    expect(document.body).not.toHaveTextContent(".5α");
    expect(
      screen.queryByText("First animal step chosen"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText(/Choose the second animal/),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Undo first step" }),
    ).not.toBeInTheDocument();
  });

  it("navigates alternative lines at the action level", () => {
    const actual = gameReducer(
      { ...newGame(initial), gameMode: "pass-and-play" },
      {
        type: "commit",
        basePositionKey: initial.positionKey,
        position: after,
        move,
      },
    );
    const alternative = {
      ...actual,
      alternativeLine: {
        divergenceIndex: 0,
        entries: [{ position: after, move: twoStepMove }],
      },
      activeLine: "alternative" as const,
    };
    localStorage.setItem(STORAGE_KEY, saveGame(alternative));
    render(<App />);

    const alternativeLog = within(
      screen.getByRole("list", { name: "Alternative Line" }),
    );
    expect(
      alternativeLog.queryByText("Alternative Line"),
    ).not.toBeInTheDocument();
    const firstAction = alternativeLog.getByRole("button", {
      name: "Go to position after 1α. Rabbit 2",
    });

    fireEvent.click(firstAction);

    expect(firstAction).toHaveAttribute("aria-current", "step");
    expect(screen.getByLabelText("Current position")).toHaveTextContent(
      "1α. Rabbit 2, …",
    );

    fireEvent.click(screen.getByRole("button", { name: "Back" }));

    expect(screen.getByLabelText("Current position")).toHaveTextContent(
      "Initial position",
    );
    expect(
      screen.getByRole("button", { name: "Go to initial position" }),
    ).toHaveAttribute("aria-current", "true");
  });

  it("confirms before replacing the main history with an alternative line", () => {
    const actual = gameReducer(
      { ...newGame(initial), gameMode: "pass-and-play" },
      {
        type: "commit",
        basePositionKey: initial.positionKey,
        position: after,
        move,
      },
    );
    const alternative = {
      ...actual,
      alternativeLine: {
        divergenceIndex: 0,
        entries: [{ position: after, move: twoStepMove }],
      },
      activeLine: "alternative" as const,
    };
    localStorage.setItem(STORAGE_KEY, saveGame(alternative));
    render(<App />);

    fireEvent.click(
      screen.getByRole("button", { name: "Use as main line" }),
    );
    const dialog = screen.getByRole("alertdialog");
    expect(dialog).toHaveTextContent("Use this as the main line?");
    expect(dialog).toHaveTextContent(
      "The main history after the branch point will be replaced by this alternative line.",
    );

    fireEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(
      screen.getByRole("list", { name: "Alternative Line" }),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("button", { name: "Use as main line" }),
    );
    fireEvent.click(
      within(screen.getByRole("alertdialog")).getByRole("button", {
        name: "Use as main line",
      }),
    );

    expect(
      screen.queryByRole("list", { name: "Alternative Line" }),
    ).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: "Go to position after 1α. Rabbit 2, Rabbit 2",
      }),
    ).toHaveAttribute("aria-current", "step");
  });

  it("shows the last completed result after the analyzer reaches its memory ceiling", async () => {
    analysisRun.mockImplementationOnce(({ requestId }, onProgress) => {
      const update = {
        ...result(requestId),
        engineName: "Garlic",
        ticks: 7_040,
        elapsedMs: 17_050,
        stoppedReason: "memory-limit" as const,
      };
      onProgress(update);
      return Promise.resolve(update);
    });
    render(<App />);

    fireEvent.click(screen.getByRole("switch", { name: "Analysis" }));

    const diagnostics = await screen.findByText(
      "Garlic · 7,040 ticks · 17.05s (OOM)",
    );
    const suggestedLine = screen.getByRole("list", {
      name: "Suggested Line",
    });
    expect(
      suggestedLine.compareDocumentPosition(diagnostics) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.queryByText(/Memory ceiling reached/)).not.toBeInTheDocument();
    expect(screen.getByText("Suggested:")).toHaveClass(
      "suggested-line__label",
    );
    expect(screen.queryByText("unreachable")).not.toBeInTheDocument();
  });

  it("places the analysis score at top left, switch at top right, and diagnostics at bottom", async () => {
    analysisRun.mockImplementationOnce(({ requestId }, onProgress) => {
      const update = {
        ...result(requestId),
        evaluation: { kind: "estimate" as const, millipoints: 23_200 },
        engineName: "Garlic",
        ticks: 7_040,
        elapsedMs: 17_050,
      };
      onProgress(update);
      return Promise.resolve(update);
    });
    render(<App />);

    const analysisSwitch = screen.getByRole("switch", { name: "Analysis" });
    fireEvent.click(analysisSwitch);

    const score = await screen.findByText("Analysis:");
    const diagnostics = screen.getByText("Garlic · 7,040 ticks · 17.05s");
    const panel = score.closest(".history-analysis");
    const toolbar = score.closest(".history-analysis__toolbar");

    expect(score.parentElement).toHaveTextContent("Analysis: +23.2");
    expect(toolbar?.lastElementChild).toContainElement(analysisSwitch);
    expect(panel?.lastElementChild).toBe(diagnostics);
  });

  it("lists the remaining strategies alphabetically", () => {
    render(<App />);
    const strategy = screen.getByLabelText("Strategy") as HTMLSelectElement;
    expect(Array.from(strategy.options, ({ text }) => text)).toEqual([
      "Avocado",
      "Cherry",
      "Fajita",
      "Garlic",
      "Iceberg",
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
