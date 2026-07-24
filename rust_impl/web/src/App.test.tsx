import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  EngineServices,
  Location,
  Position,
  TurnMove,
} from "./engine/types";

const { engine, earlier, current, move, earlierMove, analyzerAnalyze, agentChoose } = vi.hoisted(() => {
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
  const applied: Position = {
    ...earlier,
    turn: "Beta",
    turnNumber: 3,
  };
  const analyzerAnalyze = vi.fn(
    (
      request: { requestId: number },
      onProgress: (update: unknown) => void,
    ) => {
      const update = {
        requestId: request.requestId,
        bestMove: move,
        score: 125,
        depth: 3,
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
    applyMove: () => applied,
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
  return { engine: services, earlier, current, move, earlierMove, analyzerAnalyze, agentChoose };
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
    expect(screen.getByText("Version 0.9.0")).toBeInTheDocument();

    const mode = screen.getByLabelText("Mode");
    expect(mode).toHaveValue("computer-beta");
    expect(screen.getAllByRole("option").map((option) => option.textContent)).toEqual([
      "Computer plays as Alpha",
      "Computer plays as Beta",
      "Pass-and-play",
    ]);
    expect(screen.getByLabelText("Thinking Time")).toHaveValue(5);
    expect(screen.getByRole("switch", { name: "Analysis" })).not.toBeChecked();
    expect(screen.getByLabelText("Depth limit")).toHaveValue(5);
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

    expect(screen.getByLabelText("Depth limit")).toHaveValue(5);
    const score = await screen.findByText("+1.3");
    expect(score).toHaveClass("history-analysis__score--positive");
    expect(score.parentElement).not.toHaveTextContent("Alpha");
    expect(screen.getByText("Rat 2, Ox 3")).toBeInTheDocument();
    expect(screen.getByText("Best next ply")).toBeInTheDocument();
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
    expect(await screen.findByText("Ox to Rank 3")).toBeInTheDocument();
    expect(screen.getByText("Best next subply")).toBeInTheDocument();
  });

  it("constrains the analysis depth from the Game Log settings", () => {
    render(<App />);

    const depth = screen.getByLabelText("Depth limit");
    fireEvent.change(depth, { target: { value: "12" } });
    expect(depth).toHaveValue(10);
    fireEvent.change(depth, { target: { value: "0" } });
    expect(depth).toHaveValue(1);
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
});
