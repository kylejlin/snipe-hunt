import {
  Component,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
} from "react";
import type { ErrorInfo, ReactNode } from "react";
import type { MutableRefObject } from "react";
import { version } from "../package.json";
import {
  createEngineServices,
  engineInitializationError,
} from "./engine/engine-services";
import {
  type Card,
  type EngineEvaluation,
  type LiveAnalysisUpdate,
  type Location,
  type MoveStep,
  type Player,
  type Position,
  type Strategy,
  type TurnMove,
  locationLabel,
  selectionKey,
} from "./engine/types";
import {
  formatCompletedMove,
  formatCompletedStep,
  formatDisplayPlyPrefix,
  formatInitialLines,
  parseHistory,
  serializeHistory,
} from "./history-format";
import {
  activeTimeline,
  computerControls,
  gameReducer,
  type ActiveLine,
  type GameMode,
  type GameState,
  type TimelineEntry,
} from "./state/game-state";
import {
  restoreGame,
  saveGame,
  STORAGE_KEY,
} from "./state/persistence";
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
const CARD_MOVE_DURATION_MS = 200;
const strategies: ReadonlyArray<{ value: Strategy; label: string }> = [
  { value: "avocado", label: "Avocado" },
  { value: "cherry", label: "Cherry" },
  { value: "fajita", label: "Fajita" },
  { value: "garlic", label: "Garlic" },
  { value: "iceberg", label: "Iceberg" },
];

function strategyLabel(strategy: Strategy): string {
  return strategies.find(({ value }) => value === strategy)?.label ?? strategy;
}

interface MovingCard {
  animation: Animation;
  clone: HTMLButtonElement;
  card: HTMLButtonElement;
}

function sameMove(left: TurnMove | null, right: TurnMove): boolean {
  return (
    left?.positionKey === right.positionKey &&
    left.id === right.id &&
    left.player === right.player
  );
}

function sameStep(left: MoveStep | null, right: MoveStep): boolean {
  return (
    left?.pieceKey === right.pieceKey &&
    left.from === right.from &&
    left.to === right.to
  );
}

function useCardMovementAnimation(
  boardPosition: Position,
  movementOrigin: MutableRefObject<string | null>,
) {
  const boardRef = useRef<HTMLDivElement>(null);
  const previousCards = useRef<
    Array<{
      pieceKey: string;
      location: Location;
      occurrence: string;
      rect: DOMRect;
    }>
  >([]);
  const movingCards = useRef<MovingCard[]>([]);

  useLayoutEffect(() => {
    const board = boardRef.current;
    if (!board) return;

    const settleMovingCards = () => {
      for (const movement of movingCards.current) {
        movement.animation.finish();
        movement.card.style.visibility = "";
        movement.clone.remove();
      }
      movingCards.current = [];
    };

    // A newer board state wins immediately. Its animation starts from the
    // settled destination of the previous ply/subply, never from mid-flight.
    settleMovingCards();

    const cards = Array.from(
      board.querySelectorAll<HTMLButtonElement>("[data-piece-key]"),
    );
    const nextCards = cards.map((card) => ({
      element: card,
      pieceKey: card.dataset.pieceKey!,
      location: card.dataset.location as Location,
      occurrence: card.dataset.presentationOccurrence!,
      rect: card.getBoundingClientRect(),
    }));
    const preferredOrigin = movementOrigin.current;
    movementOrigin.current = null;

    const reduceMotion = window.matchMedia?.(
      "(prefers-reduced-motion: reduce)",
    ).matches;
    if (
      previousCards.current.length > 0 &&
      !reduceMotion &&
      typeof cards[0]?.animate === "function"
    ) {
      const unmatchedPrevious = [...previousCards.current];
      const moving: typeof nextCards = [];
      for (const next of nextCards) {
        const unchanged = unmatchedPrevious.findIndex(
          (previous) =>
            previous.pieceKey === next.pieceKey &&
            previous.location === next.location &&
            previous.occurrence !== preferredOrigin,
        );
        if (unchanged >= 0) {
          unmatchedPrevious.splice(unchanged, 1);
        } else {
          moving.push(next);
        }
      }
      for (const next of moving) {
        const previousIndex = unmatchedPrevious.findIndex(
          (previous) => previous.pieceKey === next.pieceKey,
        );
        if (previousIndex < 0) continue;
        const previous = unmatchedPrevious.splice(previousIndex, 1)[0];
        const card = next.element;
        const start = previous.rect;
        const end = next.rect;

        // The real card is clipped by its new lane, so animate a fixed clone
        // above the board and reveal the real card at the destination.
        const clone = card.cloneNode(true) as HTMLButtonElement;
        clone.setAttribute("aria-hidden", "true");
        clone.dataset.animationOrigin = previous.occurrence;
        clone.tabIndex = -1;
        Object.assign(clone.style, {
          position: "fixed",
          left: `${start.left}px`,
          top: `${start.top}px`,
          width: `${end.width}px`,
          height: `${end.height}px`,
          margin: "0",
          pointerEvents: "none",
          transform: "none",
          transition: "none",
          zIndex: "1000",
        });
        card.style.visibility = "hidden";
        document.body.append(clone);

        const animation = clone.animate(
          [
            { transform: "translate(0, 0)" },
            {
              transform: `translate(${end.left - start.left}px, ${end.top - start.top}px)`,
            },
          ],
          {
            duration: CARD_MOVE_DURATION_MS,
            easing: "ease-out",
            fill: "forwards",
          },
        );
        const movement = { animation, clone, card };
        movingCards.current.push(movement);
        void animation.finished
          .then(() => {
            movement.card.style.visibility = "";
            movement.clone.remove();
            movingCards.current = movingCards.current.filter(
              (candidate) => candidate !== movement,
            );
          })
          .catch(() => undefined);
      }
    }

      previousCards.current = nextCards.map(
        ({ pieceKey, location, occurrence, rect }) => ({
          pieceKey,
          location,
          occurrence,
          rect,
        }),
      );
    return settleMovingCards;
  }, [boardPosition, movementOrigin]);

  return boardRef;
}


function normalizeNumber(
  value: number,
  minimum: number,
  maximum: number,
  increment: number,
): number {
  const rounded =
    minimum + Math.round((value - minimum) / increment) * increment;
  const precision = Math.max(0, (String(increment).split(".")[1] ?? "").length);
  return Math.max(
    minimum,
    Math.min(maximum, Number(rounded.toFixed(precision))),
  );
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

        const normalized = normalizeNumber(
          numeric,
          minimum,
          maximum,
          increment,
        );
        setDraft(String(normalized));
        onCommit(normalized);
      }}
    />
  );
}

function cardImage(card: Card): string {
  return `${import.meta.env.BASE_URL}cards/${card.isSnipe ? `${card.owner}Snipe` : card.animal}.png`;
}

function cardBackground(card: Card): string {
  return `${import.meta.env.BASE_URL}cards/${card.owner}${card.canRetreat && !card.isSnipe ? "Retreater" : ""}Background.png`;
}

function CardTile({
  card,
  location,
  presentationOccurrence,
  selected,
  disabled,
  onSelect,
}: {
  card: Card;
  location: Location;
  presentationOccurrence: string;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      className={`card ${selected ? "card--selected" : ""}`}
      data-piece-key={card.pieceKey}
      data-location={location}
      data-presentation-occurrence={presentationOccurrence}
      type="button"
      aria-pressed={selected}
      aria-label={`${card.owner} ${card.animal}${card.canRetreat && !card.isSnipe ? ", retreater" : ""}`}
      onClick={onSelect}
      disabled={disabled}
    >
      <img
        aria-hidden="true"
        className="card__layer"
        src={cardBackground(card)}
        alt=""
      />
      <img
        aria-hidden="true"
        className="card__layer"
        src={cardImage(card)}
        alt=""
      />
    </button>
  );
}

function BoardLane({
  location,
  cards,
  selectedPieceKey,
  selectedOccurrence,
  selectablePieceKeys,
  legalDestination,
  interactionDisabled,
  onCardSelect,
  onDestination,
}: {
  location: Location;
  cards: Card[];
  selectedPieceKey: string | null;
  selectedOccurrence: string | null;
  selectablePieceKeys: Set<string>;
  legalDestination: boolean;
  interactionDisabled: boolean;
  onCardSelect: (selection: string, occurrence: string) => void;
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
        aria-label={
          legalDestination
            ? `Move selected card to ${locationLabel(location)}`
            : locationLabel(location)
        }
      >
        <span className="lane__rank">
          {rank ?? (location === "alpha-reserve" ? "α" : "β")}
        </span>
        <span>{isReserve ? "Reserve" : `Rank ${rank}`}</span>
        {legalDestination && <span className="lane__move-hint">Move here</span>}
      </button>
      <div className="lane__cards">
        {cards.length === 0 ? (
          <span className="lane__empty">Empty rank</span>
        ) : (
          cards.map((card, occurrence) => {
            const semanticSelection = selectionKey(card.pieceKey, location);
            const presentationOccurrence = `${semanticSelection}#${occurrence}`;
            return (
              <CardTile
                key={presentationOccurrence}
                card={card}
                location={location}
                presentationOccurrence={presentationOccurrence}
                selected={
                  selectedPieceKey === semanticSelection &&
                  selectedOccurrence === presentationOccurrence
                }
                disabled={
                  interactionDisabled ||
                  !selectablePieceKeys.has(semanticSelection)
                }
                onSelect={() =>
                  onCardSelect(semanticSelection, presentationOccurrence)
                }
              />
            );
          })
        )}
      </div>
    </section>
  );
}

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

function evaluationValue(evaluation: EngineEvaluation): number {
  if (evaluation.kind === "estimate") return evaluation.millipoints;
  return evaluation.winner === "Alpha"
    ? MATE_SCORE - evaluation.plies
    : -MATE_SCORE + evaluation.plies;
}

export function formatEvaluation(evaluation: EngineEvaluation): string {
  if (evaluation.kind === "mate") {
    return `${evaluation.winner === "Alpha" ? "+" : "-"}#${evaluation.plies}`;
  }
  const points = evaluation.millipoints / 1_000;
  return `${points >= 0 ? "+" : ""}${points.toFixed(1)}`;
}

type HistoryActionNavigation = (
  timelineIndex: number,
  stepIndex: number,
  line: ActiveLine,
) => void;

function historyLineContainsPly(
  game: GameState,
  line: ActiveLine,
  timelineIndex: number,
): boolean {
  return (
    game.activeLine === line ||
    (line === "actual" &&
      game.activeLine === "alternative" &&
      Boolean(
        game.alternativeLine &&
          timelineIndex <= game.alternativeLine.divergenceIndex,
      ))
  );
}

function historyActionIsCurrent(
  game: GameState,
  line: ActiveLine,
  timelineIndex: number,
  stepIndex: number,
  stepCount: number,
): boolean {
  if (!historyLineContainsPly(game, line, timelineIndex)) return false;
  if (stepIndex === stepCount - 1) {
    return !game.subply && game.cursor === timelineIndex;
  }
  return game.subply && game.cursor === timelineIndex - 1;
}

function scrollCurrentHistoryActionIntoView(container: HTMLOListElement) {
  const currentAction = container.querySelector<HTMLElement>("[aria-current]");
  if (!currentAction) return;

  const containerBounds = container.getBoundingClientRect();
  const actionBounds = currentAction.getBoundingClientRect();
  let nextScrollTop: number | null = null;

  if (actionBounds.top < containerBounds.top) {
    nextScrollTop = container.scrollTop + actionBounds.top - containerBounds.top;
  } else if (actionBounds.bottom > containerBounds.bottom) {
    nextScrollTop =
      container.scrollTop + actionBounds.bottom - containerBounds.bottom;
  }

  if (nextScrollTop === null) return;
  const reduceMotion = window.matchMedia?.(
    "(prefers-reduced-motion: reduce)",
  ).matches;
  container.scrollTo({
    top: Math.max(0, nextScrollTop),
    behavior: reduceMotion ? "auto" : "smooth",
  });
}

function MoveLogPly({
  timelineIndex,
  move,
  resultingWinner,
  line,
  game,
  onNavigate,
}: {
  timelineIndex: number;
  move: TurnMove;
  resultingWinner: Player | null;
  line: ActiveLine;
  game: GameState;
  onNavigate: HistoryActionNavigation;
}) {
  return (
    <li
      className={`move-list__ply move-list__ply--${move.player.toLowerCase()}`}
    >
      <small className="move-list__ply-content">
        <span className="move-list__prefix">
          {formatDisplayPlyPrefix(timelineIndex, move.player)}{" "}
        </span>
        {move.steps.map((step, stepIndex) => {
          const selected = historyActionIsCurrent(
            game,
            line,
            timelineIndex,
            stepIndex,
            move.steps.length,
          );
          const notation = formatCompletedStep(
            move,
            stepIndex,
            stepIndex === move.steps.length - 1 ? resultingWinner : null,
          );
          const positionNotation = move.steps
            .slice(0, stepIndex + 1)
            .map((_, completedStepIndex) =>
              formatCompletedStep(
                move,
                completedStepIndex,
                completedStepIndex === move.steps.length - 1
                  ? resultingWinner
                  : null,
              ),
            )
            .join(", ");
          return (
            <span key={`${step.pieceKey}-${step.from}-${step.to}-${stepIndex}`}>
              {stepIndex > 0 && <span aria-hidden="true">, </span>}
              <button
                type="button"
                className={`move-list__action${selected ? " move-list__active" : ""}`}
                aria-current={selected ? "step" : undefined}
                aria-label={`Go to position after ${formatDisplayPlyPrefix(
                  timelineIndex,
                  move.player,
                )} ${positionNotation}`}
                onClick={() => onNavigate(timelineIndex, stepIndex, line)}
              >
                {notation}
              </button>
            </span>
          );
        })}
      </small>
    </li>
  );
}

function IncompleteMoveLogPly({
  timelineIndex,
  move,
  selected,
  onNavigate,
}: {
  timelineIndex: number;
  move: TurnMove;
  selected: boolean;
  onNavigate: () => void;
}) {
  const notation = formatCompletedStep(move, 0, null);
  return (
    <li
      className={`move-list__ply move-list__ply--${move.player.toLowerCase()}`}
    >
      <small className="move-list__ply-content">
        <span className="move-list__prefix">
          {formatDisplayPlyPrefix(timelineIndex, move.player)}{" "}
        </span>
        <button
          type="button"
          className={`move-list__action${selected ? " move-list__active" : ""}`}
          aria-current={selected ? "step" : undefined}
          aria-label={`Go to position after ${formatDisplayPlyPrefix(
            timelineIndex,
            move.player,
          )} ${notation}`}
          onClick={onNavigate}
        >
          {notation}
        </button>
        <span aria-hidden="true">, …</span>
      </small>
    </li>
  );
}

function GameApp() {
  const [game, dispatch] = useReducer(
    gameReducer,
    undefined,
    () => restoreGame(localStorage.getItem(STORAGE_KEY), engine),
  );
  const gameRef = useRef(game);
  gameRef.current = game;
  const setGame = (update: (current: GameState) => GameState) => {
    const next = update(gameRef.current);
    gameRef.current = next;
    dispatch({ type: "replace", state: next });
  };
  const [selectedPieceKey, setSelectedPieceKey] = useState<string | null>(null);
  const [selectedOccurrence, setSelectedOccurrence] = useState<string | null>(
    null,
  );
  const movementOrigin = useRef<string | null>(null);
  const [analysis, setAnalysis] = useState<LiveAnalysisUpdate | null>(null);
  const [analysisRunning, setAnalysisRunning] = useState(false);
  const [agentThinking, setAgentThinking] = useState(false);
  const [agentError, setAgentError] = useState<string | null>(null);
  const [analysisError, setAnalysisError] = useState<string | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [historyMenuOpen, setHistoryMenuOpen] = useState(false);
  const [pendingConfirmation, setPendingConfirmation] = useState<
    | { kind: "reset" }
    | { kind: "import"; timeline: TimelineEntry[] }
    | null
  >(null);
  const agentRequestSequence = useRef(0);
  const analysisRequestSequence = useRef(0);
  const importInput = useRef<HTMLInputElement>(null);
  const historyMenu = useRef<HTMLDivElement>(null);
  const historyMenuButton = useRef<HTMLButtonElement>(null);
  const moveList = useRef<HTMLOListElement>(null);
  const confirmationDialog = useRef<HTMLElement>(null);
  const confirmationCancelButton = useRef<HTMLButtonElement>(null);

  const displayedTimeline = useMemo(() => activeTimeline(game), [game]);
  const entry = displayedTimeline[game.cursor];
  const position = entry.position;
  const committedMidpointMove = game.subply
    ? (displayedTimeline[game.cursor + 1]?.move ?? null)
    : null;
  const midpointStep = game.subply
    ? (committedMidpointMove?.steps[0] ?? game.draftStep)
    : null;
  const movePrefix = midpointStep ? [midpointStep] : [];
  const boardPosition = useMemo(
    () =>
      midpointStep ? engine.previewFirstStep(position, midpointStep) : position,
    [midpointStep, position],
  );
  const boardRef = useCardMovementAnimation(boardPosition, movementOrigin);
  const completedPlyCount = displayedTimeline.length - 1;
  const totalPlyCount =
    completedPlyCount + (game.draftStep ? 0.5 : 0);
  const currentPlyCount = game.cursor + (game.subply ? 0.5 : 0);
  const atPresent = currentPlyCount === totalPlyCount;
  const canMoveForward = game.subply
    ? game.cursor < displayedTimeline.length - 1
    : game.cursor < displayedTimeline.length - 1 || Boolean(game.draftStep);
  const draftBasePosition = displayedTimeline.at(-1)?.position ?? position;
  const draftMove: TurnMove | null = game.draftStep
    ? {
        id: "draft-ply",
        positionKey: draftBasePosition.positionKey,
        player: draftBasePosition.turn,
        label: "",
        steps: [game.draftStep],
        captures: game.draftStep.capture,
      }
    : null;
  const currentActionLabel = (() => {
    if (game.subply && midpointStep) {
      const partialMove = committedMidpointMove ?? draftMove;
      if (partialMove) {
        return `${formatDisplayPlyPrefix(
          game.cursor + 1,
          partialMove.player,
        )} ${formatCompletedStep(partialMove, 0, null)}, …`;
      }
    }
    if (game.cursor === 0) return "Initial position";
    const completedMove = entry.move;
    return completedMove
      ? `${formatDisplayPlyPrefix(
          game.cursor,
          completedMove.player,
        )} ${formatCompletedMove(completedMove, entry.position.winner)}`
      : "0";
  })();
  const computerTurn =
    game.activeLine === "actual" &&
    computerControls(game.gameMode, position.turn);
  const legalMoves = useMemo(() => engine.legalMoves(position), [position]);
  const prefixCandidates = legalMoves.filter((move) =>
    movePrefix.every((step, index) => {
      const candidate = move.steps[index];
      return (
        candidate?.pieceKey === step.pieceKey &&
        candidate.from === step.from &&
        candidate.to === step.to
      );
    }),
  );
  const nextStepIndex = movePrefix.length;
  const selectablePieceKeys = new Set(
    prefixCandidates
      .map((move) => move.steps[nextStepIndex])
      .filter((step): step is MoveStep => Boolean(step))
      .map((step) => selectionKey(step.pieceKey, step.from)),
  );
  const selectedMoves = selectedPieceKey
    ? prefixCandidates.filter(
        (move) => {
          const step = move.steps[nextStepIndex];
          return (
            step &&
            selectionKey(step.pieceKey, step.from) === selectedPieceKey
          );
        },
      )
    : [];
  const legalDestinations = new Set(
    selectedMoves
      .map((move) => move.steps[nextStepIndex]?.to)
      .filter((location): location is Location => Boolean(location)),
  );
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, saveGame(game));
    } catch (reason) {
      setHistoryError(
        reason instanceof Error ? reason.message : "Game could not be saved.",
      );
    }
  }, [game]);

  useEffect(() => {
    setSelectedPieceKey(null);
    setAgentError(null);
    setAnalysisError(null);
  }, [game.cursor, game.subply, position.turnNumber]);

  useLayoutEffect(() => {
    if (moveList.current) {
      scrollCurrentHistoryActionIntoView(moveList.current);
    }
  }, [
    game.activeLine,
    game.cursor,
    game.subply,
    game.timeline.length,
    game.alternativeLine?.divergenceIndex,
    game.alternativeLine?.entries.length,
    game.draftStep,
  ]);

  useEffect(() => {
    if (!historyMenuOpen) return;

    const closeOnOutsideInteraction = (event: PointerEvent) => {
      if (
        historyMenu.current &&
        !historyMenu.current.contains(event.target as Node)
      ) {
        const focusedElement = document.activeElement;
        if (
          focusedElement instanceof HTMLElement &&
          historyMenu.current.contains(focusedElement)
        ) {
          focusedElement.blur();
        }
        setHistoryMenuOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setHistoryMenuOpen(false);
      historyMenuButton.current?.focus();
    };

    document.addEventListener("pointerdown", closeOnOutsideInteraction);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnOutsideInteraction);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [historyMenuOpen]);

  useEffect(() => {
    if (!pendingConfirmation) return;

    const previouslyFocused = document.activeElement as HTMLElement | null;
    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    confirmationCancelButton.current?.focus();
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setPendingConfirmation(null);
        return;
      }
      if (event.key !== "Tab") return;

      const focusable = Array.from(
        confirmationDialog.current?.querySelectorAll<HTMLButtonElement>(
          "button:not(:disabled)",
        ) ?? [],
      );
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first || !last) return;
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("keydown", closeOnEscape);
      document.body.style.overflow = previousOverflow;
      previouslyFocused?.focus();
    };
  }, [pendingConfirmation]);

  const commitMove = (move: TurnMove) => {
    movementOrigin.current = selectedOccurrence;
    setSelectedPieceKey(null);
    setHistoryError(null);
    analysisRequestSequence.current += 1;
    setAnalysis(null);
    let nextPosition: Position;
    try {
      // Apply against the same rendered position that produced `legalMoves`.
      // React may defer or replay state updaters, so calling into WASM from the
      // updater can otherwise pair a stale move with a newer position.
      nextPosition = engine.applyMove(position, move);
    } catch (reason) {
      setHistoryError(
        reason instanceof Error
          ? reason.message
          : `Move could not be played: ${String(reason)}`,
      );
      return;
    }
    dispatch({
      type: "commit",
      basePositionKey: position.positionKey,
      position: nextPosition,
      move,
    });
  };

  useEffect(() => {
    if (
      !computerTurn ||
      !atPresent ||
      game.subply ||
      position.winner ||
      legalMoves.length === 0
    ) {
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
          timeLimitMs: Math.round(game.thinkingTimeSeconds * 1_000),
          requestId,
          strategy: game.strategy,
        },
        controller.signal,
      )
      .then((result) => {
        if (
          requestId !== agentRequestSequence.current ||
          controller.signal.aborted
        )
          return;
        // Keep rule execution out of React's updater for the same reason as
        // human moves: an updater may be replayed after the position changes.
        const nextPosition = engine.applyMove(position, result.bestMove);
        setAgentThinking(false);
        analysisRequestSequence.current += 1;
        setAnalysis(null);
        dispatch({
          type: "commit",
          basePositionKey: position.positionKey,
          position: nextPosition,
          move: result.bestMove,
        });
      })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError")
          return;
        if (requestId === agentRequestSequence.current) {
          setAgentThinking(false);
          setAgentError(
            reason instanceof Error
              ? reason.message
              : typeof reason === "string"
                ? reason
                : "Computer move failed.",
          );
        }
      });

    return () => controller.abort();
  }, [
    computerTurn,
    atPresent,
    game.subply,
    game.strategy,
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
          timeLimitMs: Math.round(game.analysisTimeSeconds * 1_000),
          requestId,
          strategy: game.strategy,
          firstStep: midpointStep ?? undefined,
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
        if (
          requestId !== analysisRequestSequence.current ||
          controller.signal.aborted
        )
          return;
        setAnalysis(result);
        setAnalysisRunning(false);
      })
      .catch((reason: unknown) => {
        if (reason instanceof DOMException && reason.name === "AbortError")
          return;
        if (requestId === analysisRequestSequence.current) {
          setAnalysisRunning(false);
          setAnalysisError(
            reason instanceof Error ? reason.message : "Analysis failed.",
          );
        }
      });

    return () => controller.abort();
  }, [
    game.analysisTimeSeconds,
    game.analysisEnabled,
    game.strategy,
    game.cursor,
    displayedTimeline,
    legalMoves.length,
    midpointStep,
    position,
  ]);

  const chooseCard = (pieceKey: string, occurrence: string) => {
    if (computerTurn || position.winner) return;
    if (!selectablePieceKeys.has(pieceKey)) return;
    if (selectedOccurrence === occurrence) {
      setSelectedPieceKey(null);
      setSelectedOccurrence(null);
    } else {
      setSelectedPieceKey(pieceKey);
      setSelectedOccurrence(occurrence);
    }
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
      movementOrigin.current = selectedOccurrence;
      setSelectedPieceKey(null);
      dispatch({ type: "draft", step: chosenStep });
    }
  };

  const moveToPosition = (nextCursor: number, activeLine: ActiveLine) => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedPieceKey(null);
    setGame((current) => {
      const nextActiveLine =
        activeLine === "alternative" && current.alternativeLine
          ? "alternative"
          : "actual";
      const nextTimeline =
        nextActiveLine === "alternative" && current.alternativeLine
          ? [
              ...current.timeline.slice(
                0,
                current.alternativeLine.divergenceIndex + 1,
              ),
              ...current.alternativeLine.entries,
            ]
          : current.timeline;
      return {
        ...current,
        activeLine: nextActiveLine,
        cursor: Math.max(0, Math.min(nextTimeline.length - 1, nextCursor)),
        subply: false,
        draftStep:
          current.activeLine === nextActiveLine ? current.draftStep : null,
      };
    });
  };

  const moveToHistoryAction: HistoryActionNavigation = (
    timelineIndex,
    stepIndex,
    activeLine,
  ) => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedPieceKey(null);
    setGame((current) => {
      const nextActiveLine =
        activeLine === "alternative" && current.alternativeLine
          ? "alternative"
          : "actual";
      const nextTimeline =
        nextActiveLine === "alternative" && current.alternativeLine
          ? [
              ...current.timeline.slice(
                0,
                current.alternativeLine.divergenceIndex + 1,
              ),
              ...current.alternativeLine.entries,
            ]
          : current.timeline;
      const move = nextTimeline[timelineIndex]?.move;
      if (!move || !move.steps[stepIndex]) return current;
      const partial = stepIndex < move.steps.length - 1;
      return {
        ...current,
        activeLine: nextActiveLine,
        cursor: partial ? timelineIndex - 1 : timelineIndex,
        subply: partial,
        draftStep:
          current.activeLine === nextActiveLine ? current.draftStep : null,
      };
    });
  };

  const moveToDraftAction = (activeLine: ActiveLine) => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedPieceKey(null);
    setGame((current) => {
      if (!current.draftStep || current.activeLine !== activeLine) return current;
      return {
        ...current,
        cursor: activeTimeline(current).length - 1,
        subply: true,
      };
    });
  };

  const moveHistoryBackward = () => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedPieceKey(null);
    setGame((current) => {
      const timeline = activeTimeline(current);
      if (current.subply) return { ...current, subply: false };
      if (current.cursor === 0) return current;
      const move = timeline[current.cursor].move;
      return {
        ...current,
        cursor: current.cursor - 1,
        subply: move?.steps.length === 2,
      };
    });
  };

  const moveHistoryForward = () => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedPieceKey(null);
    setGame((current) => {
      const timeline = activeTimeline(current);
      if (current.subply) {
        if (current.cursor >= timeline.length - 1) return current;
        return { ...current, cursor: current.cursor + 1, subply: false };
      }
      if (current.cursor < timeline.length - 1) {
        const move = timeline[current.cursor + 1].move;
        return move?.steps.length === 2
          ? { ...current, subply: true }
          : { ...current, cursor: current.cursor + 1 };
      }
      return current.draftStep ? { ...current, subply: true } : current;
    });
  };

  const performReset = () => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    const next = engine.createGame(Date.now() & 0x7fffffff);
    localStorage.removeItem(STORAGE_KEY);
    setAnalysis(null);
    setAgentThinking(false);
    setAnalysisRunning(false);
    setHistoryError(null);
    setSelectedPieceKey(null);
    setGame((current) => ({
      ...current,
      timeline: [{ position: next, move: null }],
      alternativeLine: null,
      activeLine: "actual",
      cursor: 0,
      subply: false,
      draftStep: null,
    }));
  };

  const resetGame = () => {
    setPendingConfirmation({ kind: "reset" });
  };

  const exportHistory = () => {
    setHistoryError(null);
    try {
      const exportDate = new Date();
      const computerPlayer =
        game.gameMode === "computer-alpha"
          ? "Alpha"
          : game.gameMode === "computer-beta"
            ? "Beta"
            : null;
      const contents = serializeHistory(
        game.timeline,
        {
          exportDate,
          ...(computerPlayer
            ? {
                computer: {
                  player: computerPlayer,
                  thinkingTimeSeconds: game.thinkingTimeSeconds,
                },
              }
            : {}),
        },
      );
      const blob = new Blob([contents], { type: "text/plain;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      const timestamp = exportDate
        .toISOString()
        .slice(0, 16)
        .replace(/[:T]/g, "-");
      link.href = url;
      link.download = `snipe-hunt-${timestamp}.shgh`;
      document.body.append(link);
      link.click();
      link.remove();
      URL.revokeObjectURL(url);
    } catch (reason) {
      setHistoryError(
        reason instanceof Error
          ? reason.message
          : "History could not be exported.",
      );
    }
  };

  const applyImportedHistory = (timeline: TimelineEntry[]) => {
    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAnalysis(null);
    setAgentThinking(false);
    setAnalysisRunning(false);
    setSelectedPieceKey(null);
    setGame((current) => ({
      ...current,
      timeline,
      alternativeLine: null,
      activeLine: "actual",
      cursor: timeline.length - 1,
      subply: false,
      draftStep: null,
      gameMode: "pass-and-play",
    }));
  };

  const importHistory = async (file: File) => {
    setHistoryError(null);
    if (!file.name.toLowerCase().endsWith(".shgh")) {
      setHistoryError("Choose a .shgh history file.");
      return;
    }
    try {
      const timeline = parseHistory(await file.text(), engine);
      if (game.timeline.length > 1 || game.alternativeLine) {
        setPendingConfirmation({ kind: "import", timeline });
        return;
      }
      applyImportedHistory(timeline);
    } catch (reason) {
      setHistoryError(
        reason instanceof Error
          ? reason.message
          : "History could not be imported.",
      );
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
    const moves = analysis.recommendedLine.length
      ? analysis.recommendedLine
      : [analysis.bestMove];
    let variationPosition = position;
    return moves.map((move, index) => {
      const nextVariationPosition = engine.applyMove(variationPosition, move);
      const item = {
        key: `${index}-${move.id}`,
        move,
        player: move.player,
        prefix: formatDisplayPlyPrefix(
          game.cursor + index + 1,
          move.player,
        ),
        resultingWinner: nextVariationPosition.winner,
      };
      variationPosition = nextVariationPosition;
      return item;
    });
  }, [analysis, game.cursor, position]);

  const playSuggestedLine = (
    targetMoveIndex: number,
    targetStepIndex: number,
  ) => {
    if (!analysis || targetMoveIndex < 0 || targetStepIndex < 0) return;
    const moves = analysis.recommendedLine.length
      ? analysis.recommendedLine
      : [analysis.bestMove];
    const targetMove = moves[targetMoveIndex];
    const targetStep = targetMove?.steps[targetStepIndex];
    if (!targetMove || !targetStep) return;
    const partial = targetStepIndex < targetMove.steps.length - 1;
    const selectedMoves = moves.slice(
      0,
      targetMoveIndex + (partial ? 0 : 1),
    );

    if (
      partial &&
      targetMoveIndex === 0 &&
      game.subply &&
      midpointStep &&
      sameStep(midpointStep, targetStep)
    ) {
      return;
    }

    let nextPosition = position;
    const appliedEntries: TimelineEntry[] = [];
    try {
      for (const move of selectedMoves) {
        nextPosition = engine.applyMove(nextPosition, move);
        appliedEntries.push({ position: nextPosition, move });
      }
      if (partial) engine.previewFirstStep(nextPosition, targetStep);
    } catch (reason) {
      setHistoryError(
        reason instanceof Error
          ? reason.message
          : `Suggested line could not be played: ${String(reason)}`,
      );
      return;
    }

    agentRequestSequence.current += 1;
    analysisRequestSequence.current += 1;
    setAgentThinking(false);
    setAnalysisRunning(false);
    setAnalysis(null);
    setSelectedPieceKey(null);
    setHistoryError(null);
    setGame((current) => {
      const currentTimeline = activeTimeline(current);
      if (currentTimeline[current.cursor]?.position !== position) return current;

      let matchingMoves = 0;
      while (
        matchingMoves < selectedMoves.length &&
        sameMove(
          currentTimeline[current.cursor + matchingMoves + 1]?.move ?? null,
          selectedMoves[matchingMoves],
        )
      ) {
        matchingMoves += 1;
      }

      if (
        partial &&
        matchingMoves === selectedMoves.length &&
        sameMove(
          currentTimeline[current.cursor + matchingMoves + 1]?.move ?? null,
          targetMove,
        )
      ) {
        return {
          ...current,
          cursor: current.cursor + matchingMoves,
          subply: true,
          draftStep: null,
        };
      }

      if (!partial && matchingMoves === selectedMoves.length) {
        return {
          ...current,
          cursor: current.cursor + matchingMoves,
          subply: false,
          draftStep: null,
        };
      }

      const branchIndex = current.cursor + matchingMoves;
      let divergenceIndex = branchIndex;
      let entriesBeforeSuggestion: TimelineEntry[] = [];
      if (
        current.activeLine === "alternative" &&
        current.alternativeLine &&
        branchIndex > current.alternativeLine.divergenceIndex
      ) {
        divergenceIndex = current.alternativeLine.divergenceIndex;
        entriesBeforeSuggestion = currentTimeline.slice(
          divergenceIndex + 1,
          branchIndex + 1,
        );
      }

      return {
        ...current,
        alternativeLine: {
          divergenceIndex,
          entries: [
            ...entriesBeforeSuggestion,
            ...appliedEntries.slice(matchingMoves),
          ],
        },
        activeLine: "alternative",
        cursor: current.cursor + selectedMoves.length,
        subply: partial,
        draftStep: partial ? targetStep : null,
      };
    });
  };
  const alphaEvaluation = position.winner
    ? position.winner === "Alpha"
      ? MATE_SCORE
      : -MATE_SCORE
    : analysis
      ? evaluationValue(analysis.evaluation)
      : null;
  const evaluationTone =
    alphaEvaluation === null || alphaEvaluation === 0
      ? ""
      : alphaEvaluation > 0
        ? " history-analysis__score--positive"
        : " history-analysis__score--negative";

  const renderDraftPly = (line: ActiveLine, timelineIndex: number) => {
    if (!draftMove || game.activeLine !== line) return null;
    const selected =
      game.subply &&
      game.cursor === timelineIndex - 1 &&
      committedMidpointMove === null;
    return (
      <IncompleteMoveLogPly
        key={`draft-${line}-${timelineIndex}`}
        timelineIndex={timelineIndex}
        move={draftMove}
        selected={selected}
        onNavigate={() => moveToDraftAction(line)}
      />
    );
  };

  const renderAlternativeBranch = (divergenceIndex: number) => {
    const alternative = game.alternativeLine;
    if (!alternative || alternative.divergenceIndex !== divergenceIndex) {
      return null;
    }
    const items: ReactNode[] = alternative.entries.flatMap(
      (timelineEntry, index) => {
        const timelineIndex = divergenceIndex + index + 1;
        const move = timelineEntry.move;
        if (!move) return [];
        return [
          <MoveLogPly
            key={`${move.id}-${timelineIndex}`}
            timelineIndex={timelineIndex}
            move={move}
            resultingWinner={timelineEntry.position.winner}
            line="alternative"
            game={game}
            onNavigate={moveToHistoryAction}
          />,
        ];
      },
    );
    const draftPly = renderDraftPly(
      "alternative",
      divergenceIndex + alternative.entries.length + 1,
    );
    if (draftPly) {
      items.push(draftPly);
    }

    return (
      <li
        key={`alternative-${divergenceIndex}`}
        className="move-list__alternative"
      >
        <ol aria-label="Alternative Line">{items}</ol>
      </li>
    );
  };

  return (
    <div className="app-shell">
      <header className="masthead">
        <div>
          <h1>Snipe Hunt</h1>
        </div>
        <div
          className={`turn-chip turn-chip--${position.turn.toLowerCase()}`}
          aria-live="polite"
        >
          <span className="turn-chip__dot" />
          {status}
        </div>
      </header>

      <main className="game-layout">
        <section className="table-panel" aria-label="Snipe Hunt board">
          <div className="table-panel__topline">
            <div
              className="table-panel__current-action"
              aria-label="Current position"
            >
              <strong aria-live="polite">{currentActionLabel}</strong>
            </div>
            <div className="history-controls" aria-label="History navigation">
              <button
                className="button button--quiet"
                type="button"
                onClick={moveHistoryBackward}
                disabled={game.cursor === 0 && !game.subply}
              >
                ← Back
              </button>
              <span aria-live="polite">
                {currentPlyCount} / {totalPlyCount}
              </span>
              <button
                className="button button--quiet"
                type="button"
                onClick={moveHistoryForward}
                disabled={!canMoveForward}
              >
                Forward →
              </button>
            </div>
          </div>

          {game.activeLine === "alternative" ? (
            <div className="past-banner past-banner--alternative" role="status">
              Exploring an alternative line. Actual history is unchanged.
            </div>
          ) : !atPresent ? (
            <div className="past-banner" role="status">
              Viewing an earlier position. Make a move here to begin a new line.
            </div>
          ) : null}

          <div className="board" ref={boardRef}>
            {locations.map((location) => (
              <BoardLane
                key={location}
                location={location}
                cards={boardPosition.locations[location]}
                selectedPieceKey={selectedPieceKey}
                selectedOccurrence={selectedOccurrence}
                selectablePieceKeys={selectablePieceKeys}
                legalDestination={legalDestinations.has(location)}
                interactionDisabled={computerTurn || Boolean(position.winner)}
                onCardSelect={chooseCard}
                onDestination={chooseDestination}
              />
            ))}
          </div>

          {(computerTurn || selectedPieceKey) && (
            <p className="board-help">
              {computerTurn
                ? `${position.turn} is controlled by the computer.`
                : "Choose a highlighted rank to complete the move."}
            </p>
          )}
        </section>

        <aside className="sidebar">
          <section className="control-card history-card">
            <div className="section-heading">
              <h2>Game Log</h2>
              <div className="history-heading-actions">
                <span className="move-count">
                  {completedPlyCount}{" "}
                  {completedPlyCount === 1 ? "ply" : "plies"}
                </span>
                <div className="history-menu" ref={historyMenu}>
                  <button
                    ref={historyMenuButton}
                    className="history-menu__trigger"
                    type="button"
                    aria-label="Game Log settings"
                    title="Game Log settings"
                    aria-expanded={historyMenuOpen}
                    aria-haspopup="dialog"
                    aria-controls="history-settings-menu"
                    onClick={() => setHistoryMenuOpen((open) => !open)}
                  >
                    <span aria-hidden="true">⚙</span>
                  </button>
                  {historyMenuOpen && (
                    <div
                      className="history-menu__items"
                      id="history-settings-menu"
                      role="dialog"
                      aria-label="Game Log settings"
                    >
                      <label className="field history-menu__time">
                        <span>Analysis time</span>
                        <NumericTextInput
                          value={game.analysisTimeSeconds}
                          minimum={0.25}
                          maximum={120}
                          increment={0.25}
                          ariaLabel="Analysis time"
                          onCommit={(analysisTimeSeconds) => {
                            setGame((current) => ({
                              ...current,
                              analysisTimeSeconds,
                            }));
                          }}
                        />
                        <span>seconds</span>
                      </label>
                      <div className="history-menu__divider" role="separator" />
                      <button
                        type="button"
                        onClick={() => {
                          setHistoryMenuOpen(false);
                          exportHistory();
                        }}
                      >
                        Export
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          setHistoryMenuOpen(false);
                          importInput.current?.click();
                        }}
                      >
                        Import
                      </button>
                    </div>
                  )}
                </div>
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
                          : analysis?.evaluation
                            ? formatEvaluation(analysis.evaluation)
                            : alphaEvaluation !== null
                            ? formatAlphaScore(alphaEvaluation, "Alpha")
                            : "—"}
                      </strong>
                    </div>
                  ) : (
                    <span className="history-analysis__disabled">
                      Analysis disabled
                    </span>
                  )}
                </div>
                {game.analysisEnabled && (
                  <span className="history-analysis__work">
                    {analysis?.engineName ?? strategyLabel(game.strategy)}
                    {" · "}
                    {analysis?.ticks?.toLocaleString() ?? "—"} ticks
                    {" · "}
                    {analysis?.elapsedMs !== undefined
                      ? `${(analysis.elapsedMs / 1000).toFixed(2)}s`
                      : `${game.analysisTimeSeconds}s`}
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
                      <span className="suggested-line__label">Suggested:</span>
                      {analysis.stoppedReason === "memory-limit" && (
                        <p className="history-analysis__notice" role="status">
                          Memory ceiling reached. Showing the best completed
                          result.
                        </p>
                      )}
                      <ol
                        className="suggested-line"
                        aria-label="Suggested Line"
                      >
                        {suggestedLine.map((ply, moveIndex) => {
                          const omitsPlayedFirstStep = Boolean(
                            moveIndex === 0 &&
                              game.subply &&
                              midpointStep &&
                              ply.move.steps.length > 1 &&
                              sameStep(midpointStep, ply.move.steps[0]),
                          );
                          return (
                            <li
                              key={ply.key}
                              className={`suggested-line__ply move-list__ply--${ply.player.toLowerCase()}`}
                            >
                              <span className="suggested-line__prefix">
                                {ply.prefix}{" "}
                              </span>
                              {omitsPlayedFirstStep && <span>...</span>}
                              {ply.move.steps.map((step, stepIndex) => {
                                if (omitsPlayedFirstStep && stepIndex === 0) {
                                  return null;
                                }
                                const notation = formatCompletedStep(
                                  ply.move,
                                  stepIndex,
                                  stepIndex === ply.move.steps.length - 1
                                    ? ply.resultingWinner
                                    : null,
                                );
                                const notationParts = ply.move.steps
                                  .slice(0, stepIndex + 1)
                                  .map((_, completedStepIndex) =>
                                    formatCompletedStep(
                                      ply.move,
                                      completedStepIndex,
                                      completedStepIndex ===
                                        ply.move.steps.length - 1
                                        ? ply.resultingWinner
                                        : null,
                                    ),
                                  );
                                if (omitsPlayedFirstStep) {
                                  notationParts[0] = "...";
                                }
                                const positionNotation =
                                  notationParts.join(", ");
                                return (
                                  <span
                                    key={`${step.pieceKey}-${step.from}-${step.to}-${stepIndex}`}
                                  >
                                    {stepIndex > 0 && (
                                      <span aria-hidden="true">, </span>
                                    )}
                                    <button
                                      type="button"
                                      className="suggested-line__button move-list__action"
                                      onClick={() =>
                                        playSuggestedLine(moveIndex, stepIndex)
                                      }
                                      aria-label={`Play suggested line through ${ply.prefix} ${positionNotation}`}
                                    >
                                      {notation}
                                    </button>
                                  </span>
                                );
                              })}
                            </li>
                          );
                        })}
                      </ol>
                    </>
                  ) : (
                    <p className="empty-copy">
                      {analysisRunning
                        ? `${strategyLabel(game.strategy)} is thinking…`
                        : "No legal analysis is available."}
                    </p>
                  )}
                </div>
              )}
            </div>
            <ol className="move-list" ref={moveList}>
              {game.timeline.flatMap((timelineEntry, timelineIndex) => {
                const draftPly =
                  timelineIndex === game.timeline.length - 1
                    ? renderDraftPly("actual", game.timeline.length)
                    : null;
                if (timelineIndex === 0) {
                  const initialLines = formatInitialLines(
                    timelineEntry.position,
                  );
                  const initialSelected =
                    historyLineContainsPly(game, "actual", 0) &&
                    game.cursor === 0 &&
                    !game.subply;
                  const initialPosition = (
                    <li key="initial-layout" className="move-list__layout">
                      <button
                        type="button"
                        className={initialSelected ? "move-list__active" : ""}
                        aria-current={initialSelected ? "true" : undefined}
                        aria-label="Go to initial position"
                        onClick={() => moveToPosition(0, "actual")}
                      >
                        {initialLines.map((line, index) => {
                          const player: Player =
                            index === 0 ? "Beta" : "Alpha";
                          return (
                            <small
                              key={player}
                              className={`move-list__ply--${player.toLowerCase()}`}
                            >
                              {`${formatDisplayPlyPrefix(0, player)} ${line.slice(4)}`}
                            </small>
                          );
                        })}
                      </button>
                    </li>
                  );
                  const branch = renderAlternativeBranch(0);
                  return [
                    initialPosition,
                    ...(draftPly ? [draftPly] : []),
                    ...(branch ? [branch] : []),
                  ];
                }
                const move = timelineEntry.move;
                if (!move) return [];
                const completedPly = (
                  <MoveLogPly
                    key={`${move.id}-${timelineIndex}`}
                    timelineIndex={timelineIndex}
                    move={move}
                    resultingWinner={timelineEntry.position.winner}
                    line="actual"
                    game={game}
                    onNavigate={moveToHistoryAction}
                  />
                );
                const branch = renderAlternativeBranch(timelineIndex);
                return [
                  completedPly,
                  ...(draftPly ? [draftPly] : []),
                  ...(branch ? [branch] : []),
                ];
              })}
            </ol>
          </section>

          <section className="control-card">
            <div className="section-heading">
              <h2>Game Mode</h2>
              {agentThinking && (
                <span className="thinking-spinner" aria-hidden="true" />
              )}
            </div>

            <label className="field">
              <span>Mode</span>
              <select
                value={game.gameMode}
                onChange={(event) => {
                  agentRequestSequence.current += 1;
                  setSelectedPieceKey(null);
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

            <label className="field">
              <span>Strategy</span>
              <select
                aria-label="Strategy"
                value={game.strategy}
                onChange={(event) => {
                  agentRequestSequence.current += 1;
                  analysisRequestSequence.current += 1;
                  setAnalysis(null);
                  setGame((current) => ({
                    ...current,
                    strategy: event.target.value as Strategy,
                  }));
                }}
              >
                {strategies.map(({ value, label }) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
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

            <button
              className="button button--danger"
              type="button"
              onClick={resetGame}
            >
              Reset game
            </button>
          </section>
        </aside>
      </main>

      <footer>
        <span>Version {version}</span>
      </footer>

      {pendingConfirmation && (
        <div
          className="confirmation-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) {
              setPendingConfirmation(null);
            }
          }}
        >
          <section
            ref={confirmationDialog}
            className="confirmation-dialog"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="confirmation-title"
            aria-describedby="confirmation-description"
          >
            <span className="meta-label">Please confirm</span>
            <h2 id="confirmation-title">
              {pendingConfirmation.kind === "reset"
                ? "Start a fresh game?"
                : "Import this game history?"}
            </h2>
            <p id="confirmation-description">
              The current game and its history will be replaced.
            </p>
            <div className="confirmation-dialog__actions">
              <button
                ref={confirmationCancelButton}
                className="button"
                type="button"
                onClick={() => setPendingConfirmation(null)}
              >
                Cancel
              </button>
              <button
                className="button button--confirm-danger"
                type="button"
                onClick={() => {
                  const confirmation = pendingConfirmation;
                  setPendingConfirmation(null);
                  if (confirmation.kind === "reset") {
                    performReset();
                  } else {
                    applyImportedHistory(confirmation.timeline);
                  }
                }}
              >
                {pendingConfirmation.kind === "reset"
                  ? "Reset game"
                  : "Import history"}
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

export default function App() {
  if (engineInitializationError) {
    return (
      <main className="app-shell engine-error" role="alert">
        <h1>Snipe Hunt could not start</h1>
        <p>The clean Rust/WASM engine failed to initialize.</p>
        <pre>{engineInitializationError.message}</pre>
      </main>
    );
  }
  return (
    <GameErrorBoundary>
      <GameApp />
    </GameErrorBoundary>
  );
}

class GameErrorBoundary extends Component<
  { children: ReactNode },
  { error: Error | null }
> {
  state = { error: null as Error | null };

  static getDerivedStateFromError(reason: unknown) {
    return {
      error:
        reason instanceof Error
          ? reason
          : new Error(`Unexpected game failure: ${String(reason)}`),
    };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Snipe Hunt render failed", error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <main className="app-shell engine-error" role="alert">
        <h1>Snipe Hunt hit an unexpected error</h1>
        <p>The game was stopped before any further state could be committed.</p>
        <pre>{this.state.error.message}</pre>
        <button
          className="button"
          type="button"
          onClick={() => {
            localStorage.removeItem(STORAGE_KEY);
            window.location.reload();
          }}
        >
          Discard this game and restart
        </button>
      </main>
    );
  }
}
