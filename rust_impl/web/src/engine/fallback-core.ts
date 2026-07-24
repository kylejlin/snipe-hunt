import {
  type AnalysisResult,
  type LiveAnalysisUpdate,
  type Card,
  type Location,
  type MoveStep,
  type Player,
  type Position,
  type TurnMove,
  otherPlayer,
  rowLocation,
} from "./types";

const ANIMALS = [
  "Rat",
  "Ox",
  "Tiger",
  "Rabbit",
  "Dragon",
  "Snake",
  "Horse",
  "Ram",
  "Monkey",
  "Rooster",
  "Dog",
  "Boar",
  "Fish",
  "Elephant",
  "Squid",
  "Frog",
] as const;

const RETREATERS = new Set(["Rat", "Rabbit", "Snake", "Ram", "Boar", "Squid"]);

function mulberry32(seed: number): () => number {
  let value = seed >>> 0;
  return () => {
    value += 0x6d2b79f5;
    let result = value;
    result = Math.imul(result ^ (result >>> 15), result | 1);
    result ^= result + Math.imul(result ^ (result >>> 7), result | 61);
    return ((result ^ (result >>> 14)) >>> 0) / 4294967296;
  };
}

function shuffle<T>(values: T[], seed: number): T[] {
  const random = mulberry32(seed);
  const copy = [...values];
  for (let index = copy.length - 1; index > 0; index -= 1) {
    const next = Math.floor(random() * (index + 1));
    [copy[index], copy[next]] = [copy[next], copy[index]];
  }
  return copy;
}

function makeDeck(owner: Player, seed: number): Card[] {
  return shuffle(
    ANIMALS.map((animal, index) => ({
      id: `${owner.toLowerCase()}-${animal.toLowerCase()}-${index}`,
      animal,
      owner,
      isSnipe: false,
      canRetreat: RETREATERS.has(animal),
    })),
    seed,
  );
}

export function createFallbackGame(seed = 7_071): Position {
  const alpha = makeDeck("Alpha", seed);
  const beta = makeDeck("Beta", seed ^ 0xa17e);
  const alphaSnipe: Card = {
    id: "alpha-snipe",
    animal: "Snipe",
    owner: "Alpha",
    isSnipe: true,
    canRetreat: true,
  };
  const betaSnipe: Card = {
    id: "beta-snipe",
    animal: "Snipe",
    owner: "Beta",
    isSnipe: true,
    canRetreat: true,
  };

  return {
    schemaVersion: 1,
    seed,
    turn: "Beta",
    turnNumber: 1,
    winner: null,
    locations: {
      "alpha-reserve": [alpha[0]],
      "row-1": [alpha[1], alphaSnipe, alpha[2]],
      "row-2": alpha.slice(3, 15),
      "row-3": [alpha[15]],
      "row-4": [beta[15]],
      "row-5": beta.slice(3, 15),
      "row-6": [beta[1], betaSnipe, beta[2]],
      "beta-reserve": [beta[0]],
    },
  };
}

function findCard(position: Position, cardId: string): { card: Card; location: Location } | null {
  for (const [location, cards] of Object.entries(position.locations) as [Location, Card[]][]) {
    const card = cards.find((candidate) => candidate.id === cardId);
    if (card) return { card, location };
  }
  return null;
}

function rankOf(location: Location): number | null {
  return location.startsWith("row-") ? Number(location.slice(-1)) : null;
}

function moveFor(card: Card, from: Location, to: Location): TurnMove {
  const destinationRank = rankOf(to);
  const sourceRank = rankOf(from);
  const isDrop = sourceRank === null;
  const isAdvance =
    sourceRank !== null &&
    destinationRank !== null &&
    (card.owner === "Alpha" ? destinationRank > sourceRank : destinationRank < sourceRank);
  const prefix = isDrop ? "&" : isAdvance ? "" : "*";
  return {
    id: `${card.id}:${from}:${to}`,
    player: card.owner,
    label: `${card.isSnipe ? card.owner : card.animal} ${prefix}${destinationRank}`,
    steps: [{ cardId: card.id, from, to }],
    captures: [],
  };
}

export function fallbackLegalMoves(position: Position): TurnMove[] {
  if (position.winner) return [];
  const moves: TurnMove[] = [];
  const reserve: Location = position.turn === "Alpha" ? "alpha-reserve" : "beta-reserve";

  for (const card of position.locations[reserve]) {
    for (let rank = 1; rank <= 6; rank += 1) {
      if (card.canRetreat && ((card.owner === "Alpha" && rank > 4) || (card.owner === "Beta" && rank < 3))) {
        continue;
      }
      moves.push(moveFor(card, reserve, rowLocation(rank)));
    }
  }

  for (let rank = 1; rank <= 6; rank += 1) {
    const from = rowLocation(rank);
    for (const card of position.locations[from]) {
      if (card.owner !== position.turn) continue;
      const forward = card.owner === "Alpha" ? rank + 1 : rank - 1;
      if (forward >= 1 && forward <= 6) moves.push(moveFor(card, from, rowLocation(forward)));
      const backward = card.owner === "Alpha" ? rank - 1 : rank + 1;
      if ((card.canRetreat || card.isSnipe) && backward >= 1 && backward <= 6) {
        moves.push(moveFor(card, from, rowLocation(backward)));
      }
    }
  }

  return moves.sort((left, right) => left.id.localeCompare(right.id));
}

export function applyFallbackMove(position: Position, move: TurnMove): Position {
  if (move.player !== position.turn) throw new Error("Move belongs to the wrong player.");
  if (!fallbackLegalMoves(position).some((candidate) => candidate.id === move.id)) {
    throw new Error("Move is not legal in this position.");
  }

  const locations = Object.fromEntries(
    Object.entries(position.locations).map(([location, cards]) => [location, [...cards]]),
  ) as Record<Location, Card[]>;
  let winner: Player | null = null;

  for (const step of move.steps) {
    const found = findCard({ ...position, locations }, step.cardId);
    if (!found) throw new Error("Moved card was not found.");
    locations[found.location] = locations[found.location].filter((card) => card.id !== step.cardId);

    const capturedSnipe = locations[step.to].find(
      (card) => card.isSnipe && card.owner !== found.card.owner,
    );
    if (capturedSnipe) {
      locations[step.to] = locations[step.to].filter((card) => card.id !== capturedSnipe.id);
      const winningReserve: Location =
        found.card.owner === "Alpha" ? "alpha-reserve" : "beta-reserve";
      locations[winningReserve] = [...locations[winningReserve], capturedSnipe];
      winner = found.card.owner;
    }
    locations[step.to] = [...locations[step.to], found.card];
  }

  return {
    ...position,
    locations,
    winner,
    turn: winner ? position.turn : otherPlayer(position.turn),
    turnNumber: position.turnNumber + 1,
  };
}

export function previewFallbackFirstStep(position: Position, step: MoveStep): Position {
  const legal = fallbackLegalMoves(position).some(
    (move) =>
      move.steps[0]?.cardId === step.cardId &&
      move.steps[0].from === step.from &&
      move.steps[0].to === step.to,
  );
  if (!legal) throw new Error("First animal step is not legal in this position.");

  const found = findCard(position, step.cardId);
  if (!found || found.card.isSnipe || found.location.includes("reserve")) {
    throw new Error("First subply must move an animal already on the board.");
  }

  const previewMove: TurnMove = {
    id: `${step.cardId}:${step.from}:${step.to}`,
    player: position.turn,
    label: "",
    steps: [step],
    captures: [],
  };
  const preview = applyFallbackMove(position, previewMove);
  return {
    ...preview,
    turn: position.turn,
    turnNumber: position.turnNumber,
  };
}

function scoreMove(position: Position, move: TurnMove): number {
  const next = applyFallbackMove(position, move);
  if (next.winner === position.turn) return 100_000;
  const step = move.steps[0];
  const card = findCard(position, step.cardId)?.card;
  const destinationRank = rankOf(step.to) ?? 0;
  const progress =
    position.turn === "Alpha" ? destinationRank : 7 - destinationRank;
  const enemySnipe = findCard(position, `${otherPlayer(position.turn).toLowerCase()}-snipe`);
  const enemyRank = enemySnipe ? rankOf(enemySnipe.location) : null;
  const pressure = enemyRank === destinationRank ? 5_000 : 0;
  const centrality = 4 - Math.abs(3.5 - destinationRank);
  return pressure + progress * 90 + centrality * 7 + (card?.isSnipe ? -35 : 0);
}

export function analyzeFallback(
  position: Position,
  requestId: number,
  elapsedMs: number,
  firstStep?: MoveStep,
): AnalysisResult {
  const candidates = fallbackLegalMoves(position)
    .filter((move) =>
      firstStep
        ? move.steps[0]?.cardId === firstStep.cardId &&
          move.steps[0]?.from === firstStep.from &&
          move.steps[0]?.to === firstStep.to
        : true,
    )
    .map((move) => ({ move, score: scoreMove(position, move) }))
    .sort((left, right) => right.score - left.score || left.move.id.localeCompare(right.move.id));
  const best = candidates[0];
  if (!best) throw new Error("No legal moves are available.");

  return {
    requestId,
    bestMove: best.move,
    score: best.score,
    depth: Math.max(2, Math.min(7, Math.floor(elapsedMs / 65) + 2)),
    nodes: Math.max(candidates.length, Math.floor(elapsedMs * 1_437)),
    elapsedMs,
    principalVariation: candidates.slice(0, 3).map((candidate) => candidate.move.label),
    candidates: candidates.slice(0, 4),
    engineName: "Deterministic preview engine",
  };
}

export function analyzeFallbackAtDepth(
  position: Position,
  requestId: number,
  depth: number,
  firstStep?: MoveStep,
): LiveAnalysisUpdate {
  const result = analyzeFallback(position, requestId, depth * 65, firstStep);
  const principalVariation: TurnMove[] = [];
  let variationPosition = position;
  for (let ply = 0; ply < depth && !variationPosition.winner; ply += 1) {
    const candidates = fallbackLegalMoves(variationPosition)
      .filter((move) =>
        ply === 0 && firstStep
          ? move.steps[0]?.cardId === firstStep.cardId &&
            move.steps[0]?.from === firstStep.from &&
            move.steps[0]?.to === firstStep.to
          : true,
      )
      .map((move) => ({ move, score: scoreMove(variationPosition, move) }))
      .sort((left, right) => right.score - left.score || left.move.id.localeCompare(right.move.id));
    const best = candidates[0]?.move;
    if (!best) break;
    principalVariation.push(best);
    variationPosition = applyFallbackMove(variationPosition, best);
  }
  return {
    requestId,
    bestMove: result.bestMove,
    score: result.score,
    depth,
    principalVariation,
  };
}
