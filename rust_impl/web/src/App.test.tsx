import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  EngineServices,
  Location,
  Position,
  TurnMove,
} from "./engine/types";

const {
  engine,
  earlier,
  current,
  move,
  reply,
  earlierMove,
  analyzerAnalyze,
  agentChoose,
  applyMove,
} = vi.hoisted(() => {
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
  const reply: TurnMove = {
    id: "beta-reply",
    player: "Beta",
    label: "Ox 1, Rat 2*",
    steps: [
      { cardId: "animal-1", from: "row-2", to: "row-1" },
      { cardId: "animal-0", from: "row-1", to: "row-2" },
    ],
    captures: [],
  };
  const applied: Position = {
    ...earlier,
    turn: "Beta",
    turnNumber: 3,
  };
  const analyzerAnalyze = vi.fn(
    (
      request: { requestId: number; position?: Position },
      onProgress: (update: unknown) => void,
    ) => {
      const bestMove = request.position?.turn === "Beta" ? reply : move;
      const update = {
        requestId: request.requestId,
        bestMove,
        score: 125,
        depth: 3,
        principalVariation: request.position?.turn === "Beta" ? [reply] : [move, reply],
      };
      onProgress(update);
      return Promise.resolve(update);
    },
  );
  const agentChoose = vi.fn((request: { requestId: number }) =>
    Promise.resolve({
      requestId: request.requestId,
      bestMove: move,
      score: 100,
      depth: 2,
      nodes: 100,
      elapsedMs: 5,
      principalVariation: [move.label],
      candidates: [],
      engineName: "test",
    }),
  );
  const applyMove = vi.fn((_position: Position, _move: TurnMove) => applied);
  const rules = {
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
    applyMove,
  };
  const services = {
    rules,
    computerAgent: {
      chooseMove: agentChoose,
      dispose: () => undefined,
    },
    analyzer: {
      analyze: analyzerAnalyze,
      dispose: () => undefined,
    },
  } satisfies EngineServices;
  return {
    engine: services,
    earlier,
    current,
    move,
    reply,
    earlierMove,
    analyzerAnalyze,
    agentChoose,
    applyMove,
  };
});

vi.mock("./engine/fallback-adapter", () => ({
  createEngineServices: () => engine,
}));

import App, { formatAlphaScore } from "./App";

describe("Alpha evaluation formatting", () => {
  it("rounds ordinary evaluations to one decimal from Alpha's perspective", () => {
    expect(formatAlphaScore(125, "Alpha")).toBe("+1.3");
    expect(formatAlphaScore(-66, "Alpha")).toBe("-0.7");
    expect(formatAlphaScore(66, "Beta")).toBe("-0.7");
    expect(formatAlphaScore(0, "Beta")).toBe("+0.0");
    expect(formatAlphaScore(100_000, "Alpha")).toBe("+1000.0");
  });

  it("shows forced captures as signed moves until mate", () => {
    expect(formatAlphaScore(1_000_000, "Alpha")).toBe("+#0");
    expect(formatAlphaScore(999_997, "Alpha")).toBe("+#3");
    expect(formatAlphaScore(999_998, "Beta")).toBe("-#2");
    expect(formatAlphaScore(-999_996, "Beta")).toBe("+#4");
  });
});

describe("subply history navigation", () => {
  afterEach(cleanup);

  beforeEach(() => {
    localStorage.clear();
    applyMove.mockClear();
    analyzerAnalyze.mockClear();
    agentChoose.mockClear();
    applyMove.mockImplementation((_position: Position, _move: TurnMove) => ({
      ...earlier,
      turn: "Beta",
      turnNumber: 3,
    }));
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

  it("undoes and redoes a draft first subply", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha Rat, retreater" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );
    expect(
      screen.getByRole("button", { name: "Undo first subply" }),
    ).toBeInTheDocument();
    const pendingPly = screen.getByLabelText("Subply position");
    expect(pendingPly).toHaveTextContent("1.5α.");
    expect(pendingPly).toHaveTextContent("Rat 2, …");
    expect(screen.getByText("Ply 1.5α")).toBeInTheDocument();
    expect(screen.getByText("1.5 / 1.5")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "← Back" }));

    expect(
      screen.queryByRole("button", { name: "Undo first subply" }),
    ).not.toBeInTheDocument();
    expect(screen.queryByLabelText("Subply position")).not.toBeInTheDocument();
    expect(screen.getByText("1 / 1.5")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Forward →" }));

    expect(screen.getByLabelText("Subply position")).toHaveTextContent("1.5α.");
    expect(screen.getByText("1.5 / 1.5")).toBeInTheDocument();
  });

  it("settles an active card slide before animating the next subply", () => {
    const originalAnimate = Object.getOwnPropertyDescriptor(
      HTMLElement.prototype,
      "animate",
    );
    const animations: Array<{
      finish: ReturnType<typeof vi.fn>;
      options: KeyframeAnimationOptions;
    }> = [];
    const rectSpy = vi
      .spyOn(HTMLElement.prototype, "getBoundingClientRect")
      .mockImplementation(function (this: HTMLElement) {
        const lane = this.closest<HTMLElement>(".lane")?.getAttribute("aria-label");
        const top = lane === "Rank 1" ? 100 : lane === "Rank 2" ? 200 : 0;
        return {
          x: 0,
          y: top,
          top,
          right: 50,
          bottom: top + 70,
          left: 0,
          width: 50,
          height: 70,
          toJSON: () => ({}),
        };
      });
    Object.defineProperty(HTMLElement.prototype, "animate", {
      configurable: true,
      value: vi.fn(
        (
          _keyframes: Keyframe[] | PropertyIndexedKeyframes | null,
          options: number | KeyframeAnimationOptions | undefined,
        ) => {
          const animation = {
            finish: vi.fn(),
            finished: new Promise<void>(() => undefined),
            options: options as KeyframeAnimationOptions,
          };
          animations.push(animation);
          return animation;
        },
      ),
    });

    try {
      render(<App />);
      fireEvent.click(
        screen.getByRole("button", { name: "Alpha Rat, retreater" }),
      );
      fireEvent.click(
        screen.getByRole("button", { name: "Move selected card to Rank 2" }),
      );

      expect(animations).toHaveLength(1);
      expect(animations[0].options).toMatchObject({
        duration: 200,
        fill: "forwards",
      });
      expect(document.querySelectorAll('[aria-hidden="true"][data-card-id]'))
        .toHaveLength(1);
      expect(
        document.querySelector<HTMLElement>(
          '.board [data-card-id="animal-0"]',
        ),
      ).toHaveStyle({ visibility: "hidden" });

      fireEvent.click(screen.getByRole("button", { name: "← Back" }));

      expect(animations[0].finish).toHaveBeenCalledOnce();
      expect(animations).toHaveLength(2);
      expect(document.querySelectorAll('[aria-hidden="true"][data-card-id]'))
        .toHaveLength(1);
    } finally {
      rectSpy.mockRestore();
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

  it("walks backward and forward through a committed two-step ply", () => {
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 1,
        timeline: [
          { position: earlier, move: null },
          { position: current, move: earlierMove },
        ],
        cursor: 1,
        mode: "manual",
        manualAnalysis: false,
        timeLimitSeconds: 5,
      }),
    );
    render(<App />);

    expect(screen.getByText("1 / 1")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "← Back" }));

    expect(screen.getByText("0.5 / 1")).toBeInTheDocument();
    expect(screen.getByText("Ply 1.5α")).toBeInTheDocument();
    expect(screen.getByLabelText("Subply position")).toHaveTextContent(
      "1.5α. Ox 3, …",
    );
    expect(screen.getByRole("button", { name: "Alpha Ox" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Alpha Rat, retreater" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "← Back" }));
    expect(screen.getByText("0 / 1")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Forward →" }));
    expect(screen.getByText("0.5 / 1")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Forward →" }));
    expect(screen.getByText("1 / 1")).toBeInTheDocument();
    expect(screen.queryByLabelText("Subply position")).not.toBeInTheDocument();
  });

  it("keeps one-step plies atomic", () => {
    const oneStepMove: TurnMove = {
      ...earlierMove,
      id: "one-step",
      steps: [earlierMove.steps[0]],
    };
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 1,
        timeline: [
          { position: earlier, move: null },
          { position: current, move: oneStepMove },
        ],
        cursor: 1,
        mode: "manual",
        timeLimitSeconds: 5,
      }),
    );
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "← Back" }));
    expect(screen.getByText("0 / 1")).toBeInTheDocument();
    expect(screen.queryByLabelText("Subply position")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Forward →" }));
    expect(screen.getByText("1 / 1")).toBeInTheDocument();
  });

  it("starts a new line as soon as a different first subply is played", () => {
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
        timeLimitSeconds: 5,
      }),
    );
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha Ox" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 3" }),
    );

    expect(screen.getByText("0.5 / 0.5")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Forward →" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Alpha Rat, retreater" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );

    expect(screen.getByText("1 / 1")).toBeInTheDocument();
    const stored = JSON.parse(
      localStorage.getItem("snipe-hunt.mission-7.game")!,
    ) as {
      timeline: unknown[];
      draftStep: unknown;
    };
    expect(stored.timeline).toHaveLength(2);
    expect(stored.draftStep).toBeNull();
  });

  it("analyzes a historical midpoint without starting the computer", async () => {
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 3,
        timeline: [
          { position: earlier, move: null },
          { position: current, move: earlierMove },
        ],
        cursor: 0,
        subply: true,
        draftStep: null,
        gameMode: "computer-alpha",
        thinkingTimeSeconds: 5,
        analysisEnabled: true,
        analysisDepth: 5,
      }),
    );
    render(<App />);

    await waitFor(() => {
      const request = analyzerAnalyze.mock.calls.at(-1)?.[0] as
        | { firstStep?: TurnMove["steps"][number] }
        | undefined;
      expect(request?.firstStep).toEqual(earlierMove.steps[0]);
    });
    expect(agentChoose).not.toHaveBeenCalled();
    expect(screen.getByText("0.5 / 1")).toBeInTheDocument();
  });

  it("keeps the app running and reports an engine rejection", () => {
    applyMove.mockImplementationOnce(() => {
      throw "move is not legal in this position";
    });
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "Alpha Rat, retreater" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Alpha Ox" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 3" }),
    );

    expect(screen.getByRole("alert")).toHaveTextContent(
      "Move could not be played: move is not legal in this position",
    );
    expect(screen.getByRole("heading", { name: "Snipe Hunt" })).toBeInTheDocument();
    expect(screen.getByText("1.5 / 1.5")).toBeInTheDocument();
  });

  it("persists and restores a draft midpoint", () => {
    const view = render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Alpha Rat, retreater" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );

    const stored = JSON.parse(
      localStorage.getItem("snipe-hunt.mission-7.game")!,
    ) as {
      schemaVersion: number;
      subply: boolean;
      draftStep: TurnMove["steps"][number];
    };
    expect(stored.schemaVersion).toBe(3);
    expect(stored.subply).toBe(true);
    expect(stored.draftStep).toEqual(move.steps[0]);

    view.unmount();
    render(<App />);
    expect(screen.getByText("Ply 1.5α")).toBeInTheDocument();
    expect(screen.getByText("1.5 / 1.5")).toBeInTheDocument();
  });

  it("clears stale analysis before rendering the position after a move", async () => {
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 2,
        timeline: [{ position: current, move: null }],
        cursor: 0,
        gameMode: "pass-and-play",
        thinkingTimeSeconds: 5,
        analysisEnabled: true,
        analysisDepth: 5,
      }),
    );
    applyMove.mockImplementation((position: Position, candidate: TurnMove) => {
      if (position.turn === "Beta" && candidate.player === "Alpha") {
        throw "move is not legal in this position";
      }
      return {
        ...earlier,
        turn: "Beta",
        turnNumber: 3,
      };
    });
    render(<App />);
    expect(await screen.findByText("+1.3")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Alpha Rat, retreater" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 2" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Alpha Ox" }));
    fireEvent.click(
      screen.getByRole("button", { name: "Move selected card to Rank 3" }),
    );

    expect(screen.getByText("Beta to move")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Snipe Hunt" })).toBeInTheDocument();
  });
});

describe("game mode and live analysis", () => {
  afterEach(cleanup);

  beforeEach(() => {
    localStorage.clear();
    analyzerAnalyze.mockClear();
    agentChoose.mockClear();
  });

  it("uses the new independent defaults and conditional fields", () => {
    render(<App />);

    expect(
      screen.getAllByRole("heading", { level: 2 }).map((heading) => heading.textContent),
    ).toEqual(["Game Log", "Game Mode"]);
    expect(screen.queryByText("Position")).not.toBeInTheDocument();
    expect(screen.queryByText("PLAY")).not.toBeInTheDocument();
    expect(screen.queryByText("ENGINE")).not.toBeInTheDocument();
    expect(screen.queryByText("On")).not.toBeInTheDocument();
    expect(screen.queryByText("Off")).not.toBeInTheDocument();
    expect(
      screen.queryByText(
        "Select one of the current player’s cards to see its legal destinations.",
      ),
    ).not.toBeInTheDocument();
    expect(screen.getByLabelText("Game Log settings")).toBeInTheDocument();
    expect(screen.getByText("Version 0.21.0")).toBeInTheDocument();

    const mode = screen.getByLabelText("Mode");
    expect(mode).toHaveValue("computer-beta");
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "Computer plays as Alpha",
      "Computer plays as Beta",
      "Pass-and-play",
    ]);
    expect(screen.getByLabelText("Thinking Time")).toHaveValue("5");
    expect(screen.getByRole("switch", { name: "Analysis" })).not.toBeChecked();
    expect(screen.getByText("Analysis disabled")).toBeInTheDocument();
    expect(screen.getByLabelText("Depth limit")).toHaveValue("5");
    expect(screen.queryByText("Idle")).not.toBeInTheDocument();
    expect(
      screen.queryByText("Turn on impartial, live evaluation of the displayed position."),
    ).not.toBeInTheDocument();

    fireEvent.change(mode, { target: { value: "pass-and-play" } });
    expect(screen.queryByLabelText("Thinking Time")).not.toBeInTheDocument();
  });

  it("keeps analysis informational and constrains it after the first subply", async () => {
    render(<App />);
    fireEvent.click(screen.getByRole("switch", { name: "Analysis" }));

    expect(screen.getByLabelText("Depth limit")).toHaveValue("5");
    expect(screen.queryByText("Analysis disabled")).not.toBeInTheDocument();
    const score = await screen.findByText("+1.3");
    expect(score).toHaveClass("history-analysis__score--positive");
    expect(score.parentElement).not.toHaveTextContent("Alpha");
    expect(screen.getByText("Suggested line")).toBeInTheDocument();
    const suggestedLine = screen.getByLabelText("Suggested line");
    expect(suggestedLine).toHaveClass("suggested-line");
    expect(suggestedLine).toHaveTextContent("1α.");
    expect(suggestedLine).toHaveTextContent("Rat 2, Ox 3");
    expect(suggestedLine).toHaveTextContent("1β.");
    expect(suggestedLine).toHaveTextContent("Ox 1, Rat 2*");
    expect(screen.getByText("1α. Rat 2, Ox 3")).toHaveClass(
      "move-list__ply--alpha",
    );
    expect(screen.getByText("1β. Ox 1, Rat 2*")).toHaveClass(
      "move-list__ply--beta",
    );
    expect(suggestedLine.querySelector(".move-number")).not.toBeInTheDocument();
    expect(screen.queryByText("Analyzing")).not.toBeInTheDocument();
    expect(screen.queryByText("Complete")).not.toBeInTheDocument();
    expect(screen.getByText("Depth 3 / 5")).toBeInTheDocument();
    const rat = screen.getByRole("button", { name: "Alpha Rat, retreater" });
    expect(rat).toBeEnabled();

    fireEvent.click(rat);
    fireEvent.click(screen.getByRole("button", { name: "Move selected card to Rank 2" }));

    await waitFor(() => {
      expect(analyzerAnalyze).toHaveBeenCalled();
      const request = analyzerAnalyze.mock.calls.at(-1)![0] as {
        firstStep?: TurnMove["steps"][number];
      };
      expect(request.firstStep).toEqual({
        cardId: "animal-0",
        from: "row-1",
        to: "row-2",
      });
    });
    expect(await screen.findByText("1α. Rat 2, Ox 3")).toBeInTheDocument();
    expect(screen.getByLabelText("Suggested line")).toHaveTextContent("1α.");
    expect(screen.getByLabelText("Suggested line")).toHaveTextContent("1β.");
  });

  it("falls back to the best move when the live variation is empty", async () => {
    analyzerAnalyze.mockImplementationOnce((request, onProgress) => {
      const update = {
        requestId: request.requestId,
        bestMove: move,
        score: 125,
        depth: 3,
        principalVariation: [],
      };
      onProgress(update);
      return Promise.resolve(update);
    });

    render(<App />);
    fireEvent.click(screen.getByRole("switch", { name: "Analysis" }));

    const suggestedLine = await screen.findByLabelText("Suggested line");
    expect(suggestedLine).toHaveTextContent("1α.");
    expect(suggestedLine).toHaveTextContent("Rat 2, Ox 3");
    expect(suggestedLine).not.toHaveTextContent(reply.label);
  });

  it("normalizes numeric text fields on blur", () => {
    render(<App />);

    const depth = screen.getByLabelText("Depth limit");
    const thinkingTime = screen.getByLabelText("Thinking Time");

    expect(depth).toHaveAttribute("type", "text");
    expect(depth).not.toHaveAttribute("min");
    expect(depth).not.toHaveAttribute("max");
    expect(depth).not.toHaveAttribute("step");
    expect(thinkingTime).toHaveAttribute("type", "text");
    expect(thinkingTime).not.toHaveAttribute("min");
    expect(thinkingTime).not.toHaveAttribute("max");
    expect(thinkingTime).not.toHaveAttribute("step");

    fireEvent.change(depth, { target: { value: "12" } });
    expect(depth).toHaveValue("12");
    fireEvent.blur(depth);
    expect(depth).toHaveValue("10");

    fireEvent.focus(depth);
    fireEvent.change(depth, { target: { value: "4.6" } });
    fireEvent.blur(depth);
    expect(depth).toHaveValue("5");

    fireEvent.focus(depth);
    fireEvent.change(depth, { target: { value: "not a number" } });
    fireEvent.blur(depth);
    expect(depth).toHaveValue("5");

    fireEvent.change(thinkingTime, { target: { value: "1.37" } });
    expect(thinkingTime).toHaveValue("1.37");
    fireEvent.blur(thinkingTime);
    expect(thinkingTime).toHaveValue("1.25");

    fireEvent.focus(thinkingTime);
    fireEvent.change(thinkingTime, { target: { value: "" } });
    fireEvent.blur(thinkingTime);
    expect(thinkingTime).toHaveValue("1.25");
  });

  it("migrates old manual automation to pass-and-play with analysis off", () => {
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 1,
        timeline: [{ position: current, move: null }],
        cursor: 0,
        mode: "manual",
        manualAnalysis: true,
        timeLimitSeconds: 7,
      }),
    );

    render(<App />);

    expect(screen.getByLabelText("Mode")).toHaveValue("pass-and-play");
    expect(screen.getByRole("switch", { name: "Analysis" })).not.toBeChecked();
    expect(analyzerAnalyze).not.toHaveBeenCalled();
    expect(agentChoose).not.toHaveBeenCalled();
  });

  it.each([
    ["Alpha", "+#0", "history-analysis__score--positive"],
    ["Beta", "-#0", "history-analysis__score--negative"],
  ] as const)("shows %s's terminal score without running analysis", (winner, score, tone) => {
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 2,
        timeline: [{ position: { ...current, winner }, move: null }],
        cursor: 0,
        gameMode: "pass-and-play",
        thinkingTimeSeconds: 5,
        analysisEnabled: true,
        analysisDepth: 5,
      }),
    );

    render(<App />);

    expect(screen.getByText(score)).toHaveClass(tone);
    expect(screen.queryByText("No legal analysis is available.")).not.toBeInTheDocument();
    expect(analyzerAnalyze).not.toHaveBeenCalled();
  });

  it("runs the computer agent and analyzer independently on a computer turn", async () => {
    agentChoose.mockImplementationOnce(() => new Promise(() => undefined));
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 2,
        timeline: [{ position: current, move: null }],
        cursor: 0,
        gameMode: "computer-alpha",
        thinkingTimeSeconds: 5,
        analysisEnabled: true,
        analysisDepth: 5,
      }),
    );

    render(<App />);

    await waitFor(() => {
      expect(agentChoose).toHaveBeenCalledTimes(1);
      expect(analyzerAnalyze).toHaveBeenCalled();
    });
    expect(screen.getByRole("button", { name: "Alpha Rat, retreater" })).toBeDisabled();
    expect(await screen.findByText("+1.3")).toBeInTheDocument();
  });

  it("reports an invalid computer response without crashing the app", async () => {
    applyMove.mockImplementationOnce(() => {
      throw "move is not legal in this position";
    });
    localStorage.setItem(
      "snipe-hunt.mission-7.game",
      JSON.stringify({
        schemaVersion: 2,
        timeline: [{ position: current, move: null }],
        cursor: 0,
        gameMode: "computer-alpha",
        thinkingTimeSeconds: 5,
        analysisEnabled: false,
        analysisDepth: 5,
      }),
    );

    render(<App />);

    expect(await screen.findByText("move is not legal in this position")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Snipe Hunt" })).toBeInTheDocument();
  });
});
