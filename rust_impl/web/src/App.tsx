import { useEffect, useMemo, useRef, useState } from "react";
import { version } from "../package.json";
import { createEngineAdapter } from "./engine/fallback-adapter";
import {
  type AnalysisResult,
  type Card,
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

type AnalysisMode = "alpha" | "beta" | "manual";

interface StoredGame {
  schemaVersion: 1;
  timeline: TimelineEntry[];
  cursor: number;
  mode: AnalysisMode;
  manualAnalysis: boolean;
  timeLimitSeconds: number;
}

const STORAGE_KEY = "snipe-hunt.mission-7.game";
const engine = createEngineAdapter();
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
      const parsed = JSON.parse(stored) as StoredGame;
      if (
        parsed.schemaVersion === 1 &&
        Array.isArray(parsed.timeline) &&
        parsed.timeline.length > 0 &&
        parsed.cursor >= 0 &&
        parsed.cursor < parsed.timeline.length
      ) {
        // Let the authoritative engine reject stale or malformed schemas
        // before they can crash the first render.
        engine.legalMoves(parsed.timeline[parsed.cursor].position);
        return parsed;
      }
    }
  } catch {
    localStorage.removeItem(STORAGE_KEY);
  }

  return {
    schemaVersion: 1,
    timeline: [{ position: engine.createGame(), move: null }],
    cursor: 0,
    mode: "beta",
    manualAnalysis: false,
    timeLimitSeconds: 5,
  };
}

function analysisIsOn(mode: AnalysisMode, manual: boolean, turn: Player): boolean {
  if (mode === "manual") return manual;
  return (mode === "alpha" && turn === "Alpha") || (mode === "beta" && turn === "Beta");
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

function formatScore(score: number, turn: Player): string {
  if (score >= 100_000) return `${turn} has a forced capture`;
  return `${score >= 0 ? "+" : ""}${(score / 100).toFixed(2)} for ${turn}`;
}

export default function App() {
  const [game, setGame] = useState<StoredGame>(initialState);
  const [selectedCardId, setSelectedCardId] = useState<string | null>(null);
  const [movePrefix, setMovePrefix] = useState<MoveStep[]>([]);
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [thinking, setThinking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const requestSequence = useRef(0);
  const importInput = useRef<HTMLInputElement>(null);

  const entry = game.timeline[game.cursor];
  const position = entry.position;
  const atPresent = game.cursor === game.timeline.length - 1;
  const analysisOn = analysisIsOn(game.mode, game.manualAnalysis, position.turn);
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
    setError(null);
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
    if (!analysisOn || !atPresent || position.winner || legalMoves.length === 0) {
      setThinking(false);
      return;
    }

    const requestId = ++requestSequence.current;
    const controller = new AbortController();
    setThinking(true);
    setError(null);

    engine
      .analyze(
        {
          position,
          history: game.timeline
            .slice(0, game.cursor)
            .map((timelineEntry) => timelineEntry.position),
          timeLimitMs: Math.round(game.timeLimitSeconds * 1_000),
          requestId,
        },
        controller.signal,
      )
      .then((result) => {
        if (requestId !== requestSequence.current || controller.signal.aborted) return;
        setAnalysis(result);
        setThinking(false);
        setGame((current) => {
          const isStillCurrent =
            current.cursor === current.timeline.length - 1 &&
            current.timeline[current.cursor].position.turnNumber === position.turnNumber;
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
        if (requestId === requestSequence.current) {
          setThinking(false);
          setError(reason instanceof Error ? reason.message : "Analysis failed.");
        }
      });

    return () => controller.abort();
  }, [
    analysisOn,
    atPresent,
    game.timeLimitSeconds,
    legalMoves.length,
    position,
  ]);

  const chooseCard = (cardId: string) => {
    if (thinking || analysisOn || position.winner) return;
    if (!selectableCardIds.has(cardId)) return;
    setSelectedCardId((current) => (current === cardId ? null : cardId));
  };

  const chooseDestination = (destination: Location) => {
    const matching = selectedMoves.find(
      (move) => move.steps[nextStepIndex]?.to === destination,
    );
    if (!matching) return;
    setAnalysis(null);
    const chosenStep = matching.steps[nextStepIndex];
    if (matching.steps.length === nextStepIndex + 1) {
      commitMove(matching);
    } else {
      setMovePrefix((current) => [...current, chosenStep]);
      setSelectedCardId(null);
    }
  };

  const moveCursor = (nextCursor: number) => {
    requestSequence.current += 1;
    setThinking(false);
    setAnalysis(null);
    setGame((current) => ({
      ...current,
      cursor: Math.max(0, Math.min(current.timeline.length - 1, nextCursor)),
    }));
  };

  const resetGame = () => {
    if (!window.confirm("Start a fresh game? The current history will be replaced.")) return;
    requestSequence.current += 1;
    const next = engine.createGame(Date.now() & 0x7fffffff);
    localStorage.removeItem(STORAGE_KEY);
    setAnalysis(null);
    setThinking(false);
    setHistoryError(null);
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
      requestSequence.current += 1;
      setAnalysis(null);
      setThinking(false);
      setSelectedCardId(null);
      setMovePrefix([]);
      setGame((current) => ({
        ...current,
        timeline,
        cursor: timeline.length - 1,
        mode: "manual",
        manualAnalysis: false,
      }));
    } catch (reason) {
      setHistoryError(reason instanceof Error ? reason.message : "History could not be imported.");
    }
  };

  const status = position.winner
    ? `${position.winner} wins`
    : thinking
      ? `${position.turn} is thinking…`
      : analysisOn
        ? `${position.turn} computer turn`
        : `${position.turn} to move`;

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
              <span className="meta-label">Position</span>
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
                cards={position.locations[location]}
                selectedCardId={selectedCardId}
                selectableCardIds={selectableCardIds}
                legalDestination={legalDestinations.has(location)}
                interactionDisabled={thinking || analysisOn || Boolean(position.winner)}
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
                    {cardName(position, step.cardId)} from {locationLabel(step.from)}
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
                  Cancel turn
                </button>
              </div>
            </div>
          )}

          <p className="board-help">
            {analysisOn
              ? "Analysis is active. The computer will play when its search completes."
              : selectedCardId
                ? "Choose a highlighted rank to complete the move."
                : "Select one of the current player’s cards to see its legal destinations."}
          </p>
        </section>

        <aside className="sidebar">
          <section className="control-card history-card">
            <div className="section-heading">
              <div>
                <p className="eyebrow">GAME LOG</p>
                <h2>Ply history</h2>
              </div>
              <div className="history-heading-actions">
                <span className="move-count">{game.timeline.length - 1} plies</span>
                <details className="history-menu">
                  <summary aria-label="Ply history settings" title="Ply history settings">
                    <span aria-hidden="true">⚙</span>
                  </summary>
                  <div className="history-menu__items">
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
            <ol className="move-list">
              {game.timeline.flatMap((timelineEntry, timelineIndex) => {
                if (timelineIndex === 0) {
                  return formatInitialLines(timelineEntry.position).map((line, index) => {
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
                }
                const move = timelineEntry.move;
                if (!move) return [];
                return [
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
                  </li>,
                ];
              })}
            </ol>
          </section>

          <section className="control-card">
            <div className="section-heading">
              <div>
                <p className="eyebrow">OPPONENT</p>
                <h2>Analysis control</h2>
              </div>
              <span className={`status-light ${analysisOn ? "status-light--on" : ""}`}>
                {analysisOn ? "On" : "Off"}
              </span>
            </div>

            <label className="field">
              <span>Play mode</span>
              <select
                value={game.mode}
                onChange={(event) => {
                  requestSequence.current += 1;
                  setGame((current) => ({ ...current, mode: event.target.value as AnalysisMode }));
                }}
              >
                <option value="alpha">Computer plays as Alpha</option>
                <option value="beta">Computer plays as Beta</option>
                <option value="manual">Manual</option>
              </select>
            </label>

            {game.mode === "manual" && (
              <label className="toggle-row">
                <span>
                  <strong>AI analysis</strong>
                  <small>Keep on for computer vs. computer</small>
                </span>
                <input
                  type="checkbox"
                  checked={game.manualAnalysis}
                  onChange={(event) =>
                    setGame((current) => ({ ...current, manualAnalysis: event.target.checked }))
                  }
                />
              </label>
            )}

            <label className="field">
              <span>Thinking time</span>
              <div className="time-input">
                <input
                  type="number"
                  min="0.25"
                  max="120"
                  step="0.25"
                  value={game.timeLimitSeconds}
                  onChange={(event) => {
                    const value = Number(event.target.value);
                    if (Number.isFinite(value)) {
                      setGame((current) => ({
                        ...current,
                        timeLimitSeconds: Math.max(0.25, Math.min(120, value)),
                      }));
                    }
                  }}
                />
                <span>seconds</span>
              </div>
            </label>

            <button className="button button--danger" type="button" onClick={resetGame}>
              Reset game
            </button>
          </section>

          <section className="control-card analysis-card" aria-live="polite">
            <div className="section-heading">
              <div>
                <p className="eyebrow">ENGINE ROOM</p>
                <h2>{thinking ? "Thinking…" : "Latest analysis"}</h2>
              </div>
              {thinking && <span className="thinking-spinner" aria-hidden="true" />}
            </div>

            {error ? (
              <p className="error-message">{error}</p>
            ) : analysis ? (
              <>
                <div className="evaluation">
                  <strong>{formatScore(analysis.score, analysis.bestMove.player)}</strong>
                  <span>
                    Depth {analysis.depth} · {analysis.nodes.toLocaleString()} nodes ·{" "}
                    {(analysis.elapsedMs / 1_000).toFixed(2)}s
                  </span>
                </div>
                <div className="best-line">
                  <span className="meta-label">Best line</span>
                  <ol>
                    {analysis.principalVariation.map((move, index) => (
                      <li key={`${move}-${index}`}>{move}</li>
                    ))}
                  </ol>
                </div>
                <p className="engine-note">{analysis.engineName}</p>
              </>
            ) : (
              <p className="empty-copy">
                Turn analysis on to see the engine’s evaluation, best move, and principal variation.
              </p>
            )}
          </section>

        </aside>
      </main>

      <footer>
        <span>Version {version}</span>
      </footer>
    </div>
  );
}
