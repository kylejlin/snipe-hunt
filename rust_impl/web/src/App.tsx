import { useEffect, useMemo, useRef, useState } from "react";
import { version } from "../package.json";
import { createEngineServices } from "./engine/fallback-adapter";
import {
  type Card,
  type LiveAnalysisUpdate,
  type Location,
  type MoveStep,
  type Player,
  type Position,
  type TurnMove,
  locationLabel,
} from "./engine/types";
import {
  formatDisplayPlyPrefix,
  formatInitialLines,
  formatMove,
  parseHistory,
  serializeHistory,
  type TimelineEntry,
} from "./history-format";

type GameMode = "computer-alpha" | "computer-beta" | "pass-and-play";

interface StoredGame {
  schemaVersion: 2;
  timeline: TimelineEntry[];
  cursor: number;
  gameMode: GameMode;
  thinkingTimeSeconds: number;
  analysisEnabled: boolean;
  analysisDepth: number;
}

const STORAGE_KEY = "snipe-hunt.mission-7.game";
const services = createEngineServices();
const engine = services.rules;
const locations: Location[] = [
  "alpha-reserve",
  "row-1",
  "row-2",
  "row-3",
  "row-4",
  "row-5",
  "row-6",
  "beta-reserve",
];

function initialState(): StoredGame {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as Record<string, unknown>;
      const timeline = parsed.timeline as TimelineEntry[];
      const cursor = Number(parsed.cursor);
      if (
        Array.isArray(timeline) &&
        timeline.length > 0 &&
        Number.isInteger(cursor) &&
        cursor >= 0 &&
        cursor < timeline.length
      ) {
        // Let the authoritative engine reject stale or malformed schemas
        // before they can crash the first render.
        engine.legalMoves(timeline[cursor].position);
        if (parsed.schemaVersion === 2) {
          const gameMode = parsed.gameMode;
          if (
            gameMode === "computer-alpha" ||
            gameMode === "computer-beta" ||
            gameMode === "pass-and-play"
          ) {
            return {
              schemaVersion: 2,
              timeline,
              cursor,
              gameMode,
              thinkingTimeSeconds: clampNumber(parsed.thinkingTimeSeconds, 0.25, 120, 5),
              analysisEnabled: parsed.analysisEnabled === true,
              analysisDepth: clampNumber(parsed.analysisDepth, 1, 10, 5),
            };
          }
        }
        if (parsed.schemaVersion === 1) {
          const oldMode = parsed.mode;
          return {
            schemaVersion: 2,
            timeline,
            cursor,
            gameMode:
              oldMode === "alpha"
                ? "computer-alpha"
                : oldMode === "beta"
                  ? "computer-beta"
                  : "pass-and-play",
            thinkingTimeSeconds: clampNumber(parsed.timeLimitSeconds, 0.25, 120, 5),
            analysisEnabled: false,
            analysisDepth: 5,
          };
        }
      }
    }
  } catch {
    localStorage.removeItem(STORAGE_KEY);
  }

  return {
    schemaVersion: 2,
    timeline: [{ position: engine.createGame(), move: null }],
    cursor: 0,
    gameMode: "computer-beta",
    thinkingTimeSeconds: 5,
    analysisEnabled: false,
    analysisDepth: 5,
  };
}

function clampNumber(
  value: unknown,
  minimum: number,
  maximum: number,
  fallback: number,
): number {
  const numeric = Number(value);
  return Number.isFinite(numeric)
    ? Math.max(minimum, Math.min(maximum, numeric))
    : fallback;
}

function normalizeNumber(
  value: number,
  minimum: number,
  maximum: number,
  increment: number,
): number {
  const rounded = minimum + Math.round((value - minimum) / increment) * increment;
  const precision = Math.max(0, (String(increment).split(".")[1] ?? "").length);
  return Math.max(minimum, Math.min(maximum, Number(rounded.toFixed(precision))));
}

function NumericTextInput({
  value,
  minimum,
  maximum,
  increment,
  ariaLabel,
  onCommit,
}: {
  value: number;
  minimum: number;
  maximum: number;
  increment: number;
  ariaLabel: string;
  onCommit: (value: number) => void;
}) {
  const [draft, setDraft] = useState(String(value));
  const focused = useRef(false);

  useEffect(() => {
    if (!focused.current) {
      setDraft(String(value));
    }
  }, [value]);

  return (
    <input
      type="text"
      aria-label={ariaLabel}
      value={draft}
      onFocus={() => {
        focused.current = true;
      }}
      onChange={(event) => {
        setDraft(event.target.value);
      }}
      onBlur={() => {
        focused.current = false;
        const numeric = draft.trim() === "" ? Number.NaN : Number(draft);
        if (!Number.isFinite(numeric)) {
          setDraft(String(value));
          return;
        }

        const normalized = normalizeNumber(numeric, minimum, maximum, increment);
        setDraft(String(normalized));
        onCommit(normalized);
      }}
    />
  );
}

function computerControls(mode: GameMode, turn: Player): boolean {
  return (
    (mode === "computer-alpha" && turn === "Alpha") ||
    (mode === "computer-beta" && turn === "Beta")
  );
}

function cardImage(card: Card): string {
  return `${import.meta.env.BASE_URL}cards/${card.isSnipe ? `${card.owner}Snipe` : card.animal}.png`;
}

function cardBackground(card: Card): string {
  return `${import.meta.env.BASE_URL}cards/${card.owner}${card.canRetreat && !card.isSnipe ? "Retreater" : ""}Background.png`;
}

function cardName(position: Position, cardId: string): string {
  return (
    Object.values(position.locations)
      .flat()
      .find((card) => card.id === cardId)?.animal ?? cardId
  );
}

function CardTile({
  card,
  selected,
  disabled,
  onSelect,
}: {
  card: Card;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      className={`card ${selected ? "card--selected" : ""}`}
      type="button"
      aria-pressed={selected}
      aria-label={`${card.owner} ${card.animal}${card.canRetreat && !card.isSnipe ? ", retreater" : ""}`}
      onClick={onSelect}
      disabled={disabled}
    >
      <img aria-hidden="true" className="card__layer" src={cardBackground(card)} alt="" />
      <img aria-hidden="true" className="card__layer" src={cardImage(card)} alt="" />
    </button>
  );
}

function BoardLane({
  location,
  cards,
  selectedCardId,
  selectableCardIds,
  legalDestination,
  interactionDisabled,
  onCardSelect,
  onDestination,
}: {
  location: Location;
  cards: Card[];
  selectedCardId: string | null;
  selectableCardIds: Set<string>;
  legalDestination: boolean;
  interactionDisabled: boolean;
  onCardSelect: (cardId: string) => void;
  onDestination: (location: Location) => void;
}) {
  const isReserve = location.includes("reserve");
  const rank = isReserve ? null : Number(location.slice(-1));
  return (
    <section
      className={`lane ${isReserve ? "lane--reserve" : ""} ${legalDestination ? "lane--legal" : ""}`}
      aria-label={locationLabel(location)}
    >
      <button
        className="lane__marker"
        type="button"
        onClick={() => onDestination(location)}
        disabled={!legalDestination}
        aria-label={legalDestination ? `Move selected card to ${locationLabel(location)}` : locationLabel(location)}
      >
        <span className="lane__rank">{rank ?? (location === "alpha-reserve" ? "α" : "β")}</span>
        <span>{isReserve ? "Reserve" : `Rank ${rank}`}</span>
        {legalDestination && <span className="lane__move-hint">Move here</span>}
      </button>
      <div className="lane__cards">
        {cards.length === 0 ? (
          <span className="lane__empty">Open field</span>
        ) : (
          cards.map((card) => (
            <CardTile
              key={card.id}
              card={card}
              selected={selectedCardId === card.id}
              disabled={interactionDisabled || !selectableCardIds.has(card.id)}
              onSelect={() => onCardSelect(card.id)}
            />
          ))
        )}
      </div>
    </section>
  );
}

// These mirror snipe-ai's public mate score and reserved mate-score range.
const MATE_SCORE = 1_000_000;
const MATE_THRESHOLD = MATE_SCORE - 10_000;

export function formatAlphaScore(score: number, turn: Player): string {
  const alphaScore = turn === "Alpha" ? score : -score;
  if (Math.abs(alphaScore) >= MATE_THRESHOLD) {
    const movesUntilMate = MATE_SCORE - Math.abs(alphaScore);
    return `${alphaScore >= 0 ? "+" : "-"}#${movesUntilMate}`;
  }
  return `${alphaScore >= 0 ? "+" : ""}${(alphaScore / 100).toFixed(1)}`;
}

export default function App() {
  const [game, setGame] = useState<StoredGame>(initialState);
  const [selectedCardId, setSelectedCardId] = useState<string | null>(null);
  const [movePrefix, setMovePrefix] = useState<MoveStep[]>([]);
  const [analysis, setAnalysis] = useState<LiveAnalysisUpdate | null>(null);
  const [analysisRunning, setAnalysisRunning] = useState(false);
  const [agentThinking, setAgentThinking] = useState(false);
  const [agentError, setAgentError] = useState<string | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const agentRequestSequence = useRef(0);
  const analysisRequestSequence = useRef(0);
  const importInput = useRef<HTMLInputElement>(null);

  const entry = game.timeline[game.cursor];
  const position = entry.position;
  const boardPosition = useMemo(
    () =>
      movePrefix.length > 0
        ? engine.previewFirstStep(position, movePrefix[0])
        : position,
    [movePrefix, position],
  );
  const atPresent = game.cursor === game.timeline.length - 1;
  const computerTurn = computerControls(game.gameMode, position.turn);
  const legalMoves = useMemo(() => engine.legalMoves(position), [position]);
  const prefixCandidates = legalMoves.filter((move) =>
    movePrefix.every((step, index) => {
      const candidate = move.steps[index];
      return (
        candidate?.cardId === step.cardId &&
        candidate.from === step.from &&
        candidate.to === step.to
      );
    }),
  );
  const nextStepIndex = movePrefix.length;
  const selectableCardIds = new Set(
    prefixCandidates
      .map((move) => move.steps[nextStepIndex]?.cardId)
      .filter((cardId): cardId is string => Boolean(cardId)),
  );
  const selectedMoves = selectedCardId
    ? prefixCandidates.filter(
        (move) => move.steps[nextStepIndex]?.cardId === selectedCardId,
      )
    : [];
  const legalDestinations = new Set(
    selectedMoves
      .map((move) => move.steps[nextStepIndex]?.to)
      .filter((location): location is Location => Boolean(location)),
  );
  const nextStepChoices = Array.from(
    new Map(
      prefixCandidates
        .map((move) => move.steps[nextStepIndex])
        .filter((step): step is MoveStep => Boolean(step))
        .map((step) => [`${step.cardId}:${step.from}`, step]),
    ).values(),
  );

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(game));
  }, [game]);

  useEffect(() => {
    setSelectedCardId(null);
    setMovePrefix([]);
    setAgentError(null);
    setAnalysisError(null);
  }, [game.cursor, position.turnNumber]);

  const commitMove = (move: TurnMove) => {
    setSelectedCardId(null);
    setMovePrefix([]);
    setGame((current) => {
      const active = current.timeline[current.cursor].position;
      const nextPosition = engine.applyMove(active, move);
      const timeline = current.timeline.slice(0, current.cursor + 1);
      timeline.push({ position: nextPosition, move });
      return { ...current, timeline, cursor: timeline.length - 1 };
    });
  };

  useEffect(() => {
    if (!computerTurn || !atPresent || position.winner || legalMoves.length === 0) {
      setAgentThinking(false);
      return;
    }

    const requestId = ++agentRequestSequence.current;
    const controller = new AbortController();
    setAgentThinking(true);
    setAgentError(null);

    services.computerAgent
      .chooseMove(
        {
          position,
          history: game.timeline
            .slice(0, game.cursor)
            .map((timelineEntry) => timelineEntry.position),
          timeLimitMs: Math.round(game.thinkingTimeSeconds * 1_000),
          requestId,
        },
        controller.signal,
      )
      .then((result) => {
        if (requestId !== agentRequestSequence.current || controller.signal.aborted) return;
        setAgentThinking(false);
        setGame((current) => {
          const isStillCurrent =
            current.cursor === current.timeline.length - 1 &&
            current.timeline[current.cursor].position === position &&
            computerControls(current.gameMode, position.turn);
          if (!isStillCurrent) return current;
          const nextPosition = engine.applyMove(position, result.bestMove);
          const timeline = [
            ...current.timeline,
            { position: nextPosition, move: result.bestMove },
          ];
          return { ...current, timeline, cursor: timeline.length - 1 };
        });
      })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError") return;
        if (requestId === agentRequestSequence.current) {
          setAgentThinking(false);
          setAgentError(reason instanceof Error ? reason.message : "Computer move failed.");
        }
      });

    return () => controller.abort();
  }, [
    computerTurn,
    atPresent,
    game.thinkingTimeSeconds,
    legalMoves.length,
    position,
  ]);

  useEffect(() => {
    if (!game.analysisEnabled || position.winner || legalMoves.length === 0) {
      setAnalysisRunning(false);
      if (!game.analysisEnabled) {
        setAnalysis(null);
        setAnalysisError(null);
      }
      return;
    }

    const requestId = ++analysisRequestSequence.current;
    const controller = new AbortController();
    setAnalysis(null);
    setAnalysisRunning(true);
    setAnalysisError(null);

    services.analyzer
      .analyze(
        {
          position,
          history: game.timeline
            .slice(0, game.cursor)
            .map((timelineEntry) => timelineEntry.position),
          maxDepth: game.analysisDepth,
          requestId,
          firstStep: movePrefix[0],
        },
        (update) => {
          if (
            requestId === analysisRequestSequence.current &&
            !controller.signal.aborted
          ) {
            setAnalysis(update);
          }
        },
        controller.signal,
      )
      .then((result) => {
        if (requestId !== analysisRequestSequence.current || controller.signal.aborted) return;
        setAnalysis(result);
        setAnalysisRunning(false);
      })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError") return;
        if (requestId === analysisRequestSequence.current) {
          setAnalysisRunning(false);
          setAnalysisError(reason instanceof Error ? reason.message : "Analysis failed.");
        }
      });

    return () => controller.abort();
  }, [
    game.analysisDepth,
    game.analysisEnabled,
    game.cursor,
    game.timeline,
    legalMoves.length,
    movePrefix,
    position,
  ]);

  const chooseCard = (cardId: string) => {
    if (computerTurn || position.winner) return;
    if (!selectableCardIds.has(cardId)) return;
    setSelectedCardId((current) => (current === cardId ? null : cardId));
  };

  const chooseDestination = (destination: Location) => {
    const matching = selectedMoves.find(
      (move) => move.steps[nextStepIndex]?.to === destination,
    );
    if (!matching) return;
    const chosenStep = matching.steps[nextStepIndex];
    if (matching.steps.length === nextStepIndex + 1) {
      commitMove(matching);
    } else {
      setMovePrefix((current) => [...current, chosenStep]);
      setSelectedCardId(null);
    }
  };

  const moveCursor = (nextCursor: number) => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedCardId(null);
    setMovePrefix([]);
    setGame((current) => ({
      ...current,
      cursor: Math.max(0, Math.min(current.timeline.length - 1, nextCursor)),
    }));
  };

  const resetGame = () => {
    if (!window.confirm("Start a fresh game? The current history will be replaced.")) return;
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    const next = engine.createGame(Date.now() & 0x7fffffff);
    localStorage.removeItem(STORAGE_KEY);
    setAnalysis(null);
    setAgentThinking(false);
    setAnalysisRunning(false);
    setHistoryError(null);
    setSelectedCardId(null);
    setMovePrefix([]);
    setGame((current) => ({
      ...current,
      timeline: [{ position: next, move: null }],
      cursor: 0,
    }));
  };

  const exportHistory = () => {
    setHistoryError(null);
    try {
      const contents = serializeHistory(game.timeline);
      const blob = new Blob([contents], { type: "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      const timestamp = new Date().toISOString().slice(0, 16).replace(/[:T]/g, "-");
      link.href = url;
      link.download = `snipe-hunt-${timestamp}.shgh`;
      document.body.append(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
    } catch (reason) {
      setHistoryError(reason instanceof Error ? reason.message : "History could not be exported.");
    }
  };

  const importHistory = async (file: File) => {
    setHistoryError(null);
    if (!file.name.toLowerCase().endsWith(".shgh")) {
      setHistoryError("Choose a .shgh history file.");
      return;
    }
    try {
      const timeline = parseHistory(await file.text(), engine);
      if (
        game.timeline.length > 1 &&
        !window.confirm("Import this history? The current game will be replaced.")
      ) {
        return;
      }
      agentRequestSequence.current += 1;
      analysisRequestSequence.current += 1;
      setAnalysis(null);
      setAgentThinking(false);
      setAnalysisRunning(false);
      setSelectedCardId(null);
      setMovePrefix([]);
      setGame((current) => ({
        ...current,
        timeline,
        cursor: timeline.length - 1,
        gameMode: "pass-and-play",
      }));
    } catch (reason) {
      setHistoryError(reason instanceof Error ? reason.message : "History could not be imported.");
    }
  };

  const status = position.winner
    ? `${position.winner} wins`
    : agentThinking
      ? `${position.turn} is thinking…`
      : computerTurn
        ? `${position.turn} computer turn`
        : `${position.turn} to move`;
  const suggestedLine = useMemo(() => {
    if (!analysis) return [];
    const moves =
      analysis.principalVariation.length > 0
        ? analysis.principalVariation
        : [analysis.bestMove];
    let variationPosition = position;
    return moves.map((move, index) => {
      const item = {
        key: `${index}-${move.id}`,
        player: move.player,
        prefix: formatDisplayPlyPrefix(
          Math.ceil((game.cursor + index + 1) / 2),
          move.player,
        ),
        notation: formatMove(variationPosition, move),
      };
      variationPosition = engine.applyMove(variationPosition, move);
      return item;
    });
  }, [analysis, game.cursor, position]);
  const alphaEvaluation = position.winner
    ? position.winner === "Alpha"
      ? MATE_SCORE
      : -MATE_SCORE
    : analysis
      ? position.turn === "Alpha"
        ? analysis.score
        : -analysis.score
      : null;
  const evaluationTone =
    alphaEvaluation === null || alphaEvaluation === 0
      ? ""
      : alphaEvaluation > 0
        ? " history-analysis__score--positive"
        : " history-analysis__score--negative";

  return (
    <div className="app-shell">
      <header className="masthead">
        <div>
          <h1>Snipe Hunt</h1>
        </div>
        <div className={`turn-chip turn-chip--${position.turn.toLowerCase()}`} aria-live="polite">
          <span className="turn-chip__dot" />
          {status}
        </div>
      </header>

      <main className="game-layout">
        <section className="table-panel" aria-label="Snipe Hunt board">
          <div className="table-panel__topline">
            <div>
              <strong>
                Ply{" "}
                {formatDisplayPlyPrefix(
                  Math.ceil(position.turnNumber / 2),
                  position.turn,
                ).slice(0, -1)}
              </strong>
            </div>
            <div className="history-controls" aria-label="History navigation">
              <button
                className="button button--quiet"
                type="button"
                onClick={() => moveCursor(game.cursor - 1)}
                disabled={game.cursor === 0}
              >
                ← Back
              </button>
              <span aria-live="polite">
                {game.cursor + 1} / {game.timeline.length}
              </span>
              <button
                className="button button--quiet"
                type="button"
                onClick={() => moveCursor(game.cursor + 1)}
                disabled={atPresent}
              >
                Forward →
              </button>
            </div>
          </div>

          {!atPresent && (
            <div className="past-banner" role="status">
              Viewing an earlier position. Make a move here to begin a new line.
            </div>
          )}

          <div className="board">
            {locations.map((location) => (
              <BoardLane
                key={location}
                location={location}
                cards={boardPosition.locations[location]}
                selectedCardId={selectedCardId}
                selectableCardIds={selectableCardIds}
                legalDestination={legalDestinations.has(location)}
                interactionDisabled={computerTurn || Boolean(position.winner)}
                onCardSelect={chooseCard}
                onDestination={chooseDestination}
              />
            ))}
          </div>

          {movePrefix.length > 0 && (
            <div className="turn-builder" role="status">
              <div>
                <strong>First animal step chosen</strong>
                <span>
                  {locationLabel(movePrefix[0].from)} → {locationLabel(movePrefix[0].to)}.
                  Choose the second animal.
                </span>
              </div>
              <div className="turn-builder__choices" aria-label="Legal second animals">
                {nextStepChoices.map((step) => (
                  <button
                    className="button button--quiet"
                    type="button"
                    key={`${step.cardId}:${step.from}`}
                    onClick={() => setSelectedCardId(step.cardId)}
                  >
                    {cardName(boardPosition, step.cardId)} from {locationLabel(step.from)}
                  </button>
                ))}
                <button
                  className="button button--quiet"
                  type="button"
                  onClick={() => {
                    setMovePrefix([]);
                    setSelectedCardId(null);
                  }}
                >
                  Undo first subply
                </button>
              </div>
            </div>
          )}

          {(computerTurn || selectedCardId || movePrefix.length > 0) && (
            <p className="board-help">
              {computerTurn
                ? `${position.turn} is controlled by the computer.`
                : selectedCardId
                  ? "Choose a highlighted rank to complete the move."
                  : "Choose the second animal to complete this ply."}
            </p>
          )}
        </section>

        <aside className="sidebar">
          <section className="control-card history-card">
            <div className="section-heading">
              <h2>Game Log</h2>
              <div className="history-heading-actions">
                <span className="move-count">{game.timeline.length - 1} plies</span>
                <details className="history-menu">
                  <summary aria-label="Game Log settings" title="Game Log settings">
                    <span aria-hidden="true">⚙</span>
                  </summary>
                  <div className="history-menu__items">
                    <label className="field history-menu__depth">
                      <span>Analysis depth</span>
                      <NumericTextInput
                        value={game.analysisDepth}
                        minimum={1}
                        maximum={10}
                        increment={1}
                        ariaLabel="Depth limit"
                        onCommit={(analysisDepth) => {
                          setGame((current) => ({
                            ...current,
                            analysisDepth,
                          }));
                        }}
                      />
                    </label>
                    <div className="history-menu__divider" role="separator" />
                    <button type="button" onClick={exportHistory}>
                      Export
                    </button>
                    <button type="button" onClick={() => importInput.current?.click()}>
                      Import
                    </button>
                  </div>
                </details>
              </div>
            </div>
            <input
              ref={importInput}
              className="visually-hidden"
              type="file"
              accept=".shgh"
              aria-label="Choose a Snipe Hunt history file"
              onChange={(event) => {
                const file = event.target.files?.[0];
                event.target.value = "";
                if (file) void importHistory(file);
              }}
            />
            {historyError && (
              <p className="error-message history-error" role="alert">
                {historyError}
              </p>
            )}
            <div
              className={`history-analysis${game.analysisEnabled ? " history-analysis--enabled" : ""}`}
            >
              <div className="history-analysis__toolbar" aria-live="polite">
                <div className="history-analysis__summary">
                  <label className="analysis-switch">
                    <input
                      type="checkbox"
                      role="switch"
                      aria-label="Analysis"
                      checked={game.analysisEnabled}
                      onChange={(event) => {
                        analysisRequestSequence.current += 1;
                        setGame((current) => ({
                          ...current,
                          analysisEnabled: event.target.checked,
                        }));
                      }}
                    />
                  </label>
                  {game.analysisEnabled ? (
                    <div className="history-analysis__score">
                      <strong className={evaluationTone}>
                        {analysisError
                          ? "—"
                          : alphaEvaluation !== null
                            ? formatAlphaScore(alphaEvaluation, "Alpha")
                              : "—"}
                      </strong>
                    </div>
                  ) : (
                    <span className="history-analysis__disabled">Analysis disabled</span>
                  )}
                </div>
                {game.analysisEnabled && (
                  <span className="history-analysis__depth">
                    Depth {analysis?.depth ?? "—"} / {game.analysisDepth}
                  </span>
                )}
              </div>
              {game.analysisEnabled && (
                <div className="history-analysis__advice" aria-live="polite">
                  {analysisError ? (
                    <p className="error-message">{analysisError}</p>
                  ) : position.winner ? (
                    <>
                      <span className="meta-label">Best next ply</span>
                      <strong>Game complete</strong>
                    </>
                  ) : analysis ? (
                    <>
                      <span className="meta-label">Suggested line</span>
                      <ol className="suggested-line" aria-label="Suggested line">
                        {suggestedLine.map((ply) => (
                          <li
                            key={ply.key}
                            className={`suggested-line__ply move-list__ply--${ply.player.toLowerCase()}`}
                          >
                            <span className="move-number">{ply.prefix}</span>
                            <span className="suggested-line__move">{ply.notation}</span>
                          </li>
                        ))}
                      </ol>
                    </>
                  ) : (
                    <p className="empty-copy">
                      {analysisRunning
                        ? "Searching for the first completed depth…"
                        : "No legal analysis is available."}
                    </p>
                  )}
                </div>
              )}
            </div>
            <ol className="move-list">
              {game.timeline.flatMap((timelineEntry, timelineIndex) => {
                const pendingSubply =
                  timelineIndex === game.cursor && movePrefix.length > 0 ? (
                    <li
                      key="pending-subply"
                      className={`move-list__pending move-list__ply--${position.turn.toLowerCase()}`}
                      aria-label="Pending ply"
                    >
                      <div>
                        <span className="move-number">
                          {formatDisplayPlyPrefix(
                            Math.ceil(position.turnNumber / 2),
                            position.turn,
                          )}
                        </span>
                        <small>
                          {formatMove(position, {
                            id: "pending-subply",
                            player: position.turn,
                            label: "",
                            steps: movePrefix,
                            captures: [],
                          })}
                          , ...
                        </small>
                      </div>
                    </li>
                  ) : null;
                if (timelineIndex === 0) {
                  const initialLines = formatInitialLines(timelineEntry.position).map((line, index) => {
                    const player: Player = index === 0 ? "Beta" : "Alpha";
                    return (
                      <li
                        key={`initial-layout-${player.toLowerCase()}`}
                        className={`move-list__layout move-list__ply--${player.toLowerCase()}`}
                      >
                        <button
                          type="button"
                          className={game.cursor === 0 ? "move-list__active" : ""}
                          onClick={() => moveCursor(0)}
                        >
                          <span className="move-number">
                            {formatDisplayPlyPrefix(0, player)}
                          </span>
                          <small>{line.slice(4)}</small>
                        </button>
                      </li>
                    );
                  });
                  return pendingSubply ? [...initialLines, pendingSubply] : initialLines;
                }
                const move = timelineEntry.move;
                if (!move) return [];
                const completedPly = (
                  <li
                    key={`${move.id}-${timelineIndex}`}
                    className={`move-list__ply--${move.player.toLowerCase()}`}
                  >
                    <button
                      type="button"
                      className={game.cursor === timelineIndex ? "move-list__active" : ""}
                      onClick={() => moveCursor(timelineIndex)}
                    >
                      <span className="move-number">
                        {formatDisplayPlyPrefix(Math.ceil(timelineIndex / 2), move.player)}
                      </span>
                      <small>
                        {formatMove(game.timeline[timelineIndex - 1].position, move)}
                      </small>
                    </button>
                  </li>
                );
                return pendingSubply ? [completedPly, pendingSubply] : [completedPly];
              })}
            </ol>
          </section>

          <section className="control-card">
            <div className="section-heading">
              <h2>Game Mode</h2>
              {agentThinking && <span className="thinking-spinner" aria-hidden="true" />}
            </div>

            <label className="field">
              <span>Mode</span>
              <select
                value={game.gameMode}
                onChange={(event) => {
                  agentRequestSequence.current += 1;
                  setSelectedCardId(null);
                  setMovePrefix([]);
                  setGame((current) => ({
                    ...current,
                    gameMode: event.target.value as GameMode,
                  }));
                }}
              >
                <option value="computer-alpha">Computer plays as Alpha</option>
                <option value="computer-beta">Computer plays as Beta</option>
                <option value="pass-and-play">Pass-and-play</option>
              </select>
            </label>

            {game.gameMode !== "pass-and-play" && (
              <label className="field">
                <span>Thinking Time</span>
                <div className="time-input">
                  <NumericTextInput
                    value={game.thinkingTimeSeconds}
                    minimum={0.25}
                    maximum={120}
                    increment={0.25}
                    ariaLabel="Thinking Time"
                    onCommit={(thinkingTimeSeconds) => {
                      setGame((current) => ({
                        ...current,
                        thinkingTimeSeconds,
                      }));
                    }}
                  />
                  <span>seconds</span>
                </div>
              </label>
            )}

            {agentError && <p className="error-message">{agentError}</p>}

            <button className="button button--danger" type="button" onClick={resetGame}>
              Reset game
            </button>
          </section>

        </aside>
      </main>

      <footer>
        <span>Version {version}</span>
      </footer>
    </div>
  );
}
