import type {
  Card,
  EngineAdapter,
  Location,
  Player,
  Position,
  TurnMove,
} from "./engine/types";

export interface TimelineEntry {
  position: Position;
  move: TurnMove | null;
}

const ANIMAL_NAMES = [
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

const ANIMAL_INDEX = new Map<string, number>(
  ANIMAL_NAMES.map((name, index) => [name, index]),
);
const RETREATERS = new Set(["Rat", "Rabbit", "Snake", "Ram", "Boar", "Squid"]);

function rankOf(location: Location): number | null {
  return location.startsWith("row-") ? Number(location.slice(4)) : null;
}

function cardAt(position: Position, cardId: string): Card {
  const card = Object.values(position.locations)
    .flat()
    .find((candidate) => candidate.id === cardId);
  if (!card) throw new Error(`Card ${cardId} is missing from the position.`);
  return card;
}

function cardNotation(card: Card): string {
  return card.isSnipe ? card.owner : card.animal;
}

function stepNotation(position: Position, move: TurnMove, stepIndex: number): string {
  const step = move.steps[stepIndex];
  if (!step) throw new Error("Move contains an empty step.");
  const card = cardAt(position, step.cardId);
  const destinationRank = rankOf(step.to);
  if (destinationRank === null) throw new Error("Moves into a reserve cannot be serialized.");

  if (step.from.includes("reserve")) {
    return `${cardNotation(card)} ${destinationRank}!`;
  }

  const sourceRank = rankOf(step.from);
  if (sourceRank === null) throw new Error("Move source is invalid.");
  const isAdvance =
    move.player === "Alpha"
      ? destinationRank > sourceRank
      : destinationRank < sourceRank;
  return `${cardNotation(card)} ${destinationRank}${isAdvance ? "" : "*"}`;
}

export function formatMove(position: Position, move: TurnMove): string {
  return move.steps
    .map((_, stepIndex) => stepNotation(position, move, stepIndex))
    .join(", ");
}

export function snipeCaptureSuffix(
  before: Position,
  after: Position,
): "" | "+#0" | "-#0" {
  const capturedInto = (reserve: "alpha-reserve" | "beta-reserve", owner: Player) => {
    const containsSnipe = (position: Position) =>
      position.locations[reserve].some(
        (card) => card.isSnipe && card.owner === owner,
      );
    return !containsSnipe(before) && containsSnipe(after);
  };

  if (capturedInto("alpha-reserve", "Beta")) return "+#0";
  if (capturedInto("beta-reserve", "Alpha")) return "-#0";
  return "";
}

export function formatCompletedMove(
  before: Position,
  move: TurnMove,
  after: Position,
): string {
  const notation = formatMove(before, move);
  const suffix = snipeCaptureSuffix(before, after);
  return suffix ? `${notation} ${suffix}` : notation;
}

export function formatPlyPrefix(timelineIndex: number, player: Player): string {
  if (timelineIndex < 1) throw new Error("Move plies begin at timeline index 1.");
  return `${Math.ceil(timelineIndex / 2)}${player === "Alpha" ? "a" : "b"}.`;
}

export function formatDisplayPlyPrefix(
  plyNumber: number,
  player: Player,
  firstSubplyPlayed = false,
): string {
  if (plyNumber < 0) throw new Error("Ply numbers cannot be negative.");
  return `${plyNumber}${firstSubplyPlayed ? ".5" : ""}${player === "Alpha" ? "α" : "β"}.`;
}

function formatLocation(position: Position, location: Location): string {
  return position.locations[location].map(cardNotation).join(" ");
}

export function formatInitialLines(position: Position): [string, string] {
  return [
    `0b. =${[
      "beta-reserve",
      "row-6",
      "row-5",
      "row-4",
    ].map((location) => formatLocation(position, location as Location)).join("; ")}`,
    `0a. =${[
      "alpha-reserve",
      "row-1",
      "row-2",
      "row-3",
    ].map((location) => formatLocation(position, location as Location)).join("; ")}`,
  ];
}

export function serializeHistory(timeline: TimelineEntry[]): string {
  if (timeline.length === 0) throw new Error("A history must contain an initial position.");
  const lines = [...formatInitialLines(timeline[0].position)];
  for (let index = 1; index < timeline.length; index += 1) {
    const entry = timeline[index];
    if (!entry.move) throw new Error(`Timeline entry ${index} has no move.`);
    lines.push(
      `${formatPlyPrefix(index, entry.move.player)} ${formatCompletedMove(
        timeline[index - 1].position,
        entry.move,
        entry.position,
      )}`,
    );
  }
  return `${lines.join("\n")}\n`;
}

function parseLayoutLine(
  line: string,
  lineNumber: number,
  player: Player,
  expectedPrefix: string,
): string[][] {
  if (!line.startsWith(`${expectedPrefix} =`)) {
    throw new Error(`Line ${lineNumber}: expected "${expectedPrefix} =".`);
  }
  const groups = line.slice(expectedPrefix.length + 2).split(";").map((group) => group.trim());
  if (groups.length !== 4 || groups.some((group) => group.length === 0)) {
    throw new Error(`Line ${lineNumber}: the initial layout must contain four nonempty groups.`);
  }
  const tokens = groups.map((group) => group.split(/\s+/));
  const expectedCounts = [1, 3, 12, 1];
  tokens.forEach((group, index) => {
    if (group.length !== expectedCounts[index]) {
      throw new Error(
        `Line ${lineNumber}: layout group ${index + 1} must contain ${expectedCounts[index]} card${expectedCounts[index] === 1 ? "" : "s"}.`,
      );
    }
  });
  const snipeName = player;
  const otherSnipe = player === "Alpha" ? "Beta" : "Alpha";
  if (tokens[1].filter((name) => name === snipeName).length !== 1) {
    throw new Error(`Line ${lineNumber}: ${snipeName} must appear once in the home rank.`);
  }
  if (
    tokens.flat().some((name) => name === otherSnipe) ||
    tokens.flat().filter((name) => name === snipeName).length !== 1
  ) {
    throw new Error(`Line ${lineNumber}: the snipe layout is invalid.`);
  }
  for (const name of tokens.flat()) {
    if (name !== snipeName && !ANIMAL_INDEX.has(name)) {
      throw new Error(`Line ${lineNumber}: unknown card "${name}".`);
    }
  }
  return tokens;
}

function cardFromName(
  name: string,
  owner: Player,
  occurrences: Map<string, number>,
): Card {
  if (name === owner) {
    return {
      id: `${owner.toLowerCase()}-snipe`,
      animal: "Snipe",
      owner,
      isSnipe: true,
      canRetreat: true,
    };
  }
  const baseIndex = ANIMAL_INDEX.get(name);
  if (baseIndex === undefined) throw new Error(`Unknown animal "${name}".`);
  const occurrence = occurrences.get(name) ?? 0;
  if (occurrence >= 2) throw new Error(`The initial layout contains more than two ${name} cards.`);
  occurrences.set(name, occurrence + 1);
  return {
    id: `animal-${baseIndex + occurrence * 16}`,
    animal: name,
    owner,
    isSnipe: false,
    canRetreat: RETREATERS.has(name),
  };
}

function parseInitialPosition(
  betaLine: string,
  betaLineNumber: number,
  alphaLine: string,
  alphaLineNumber: number,
): Position {
  const beta = parseLayoutLine(betaLine, betaLineNumber, "Beta", "0b.");
  const alpha = parseLayoutLine(alphaLine, alphaLineNumber, "Alpha", "0a.");
  const occurrences = new Map<string, number>();
  const cards = (names: string[], owner: Player) =>
    names.map((name) => cardFromName(name, owner, occurrences));

  const position: Position = {
    schemaVersion: 1,
    seed: 0,
    turn: "Beta",
    turnNumber: 1,
    winner: null,
    locations: {
      "alpha-reserve": cards(alpha[0], "Alpha"),
      "row-1": cards(alpha[1], "Alpha"),
      "row-2": cards(alpha[2], "Alpha"),
      "row-3": cards(alpha[3], "Alpha"),
      "row-4": cards(beta[3], "Beta"),
      "row-5": cards(beta[2], "Beta"),
      "row-6": cards(beta[1], "Beta"),
      "beta-reserve": cards(beta[0], "Beta"),
    },
  };
  for (const name of ANIMAL_NAMES) {
    if (occurrences.get(name) !== 2) {
      throw new Error(`Initial layout must contain exactly two ${name} cards.`);
    }
  }
  return position;
}

export function parseHistory(
  source: string,
  engine: Pick<EngineAdapter, "legalMoves" | "applyMove">,
): TimelineEntry[] {
  const lines = source
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((text, index) => ({ text, lineNumber: index + 1 }))
    .filter(({ text }) => text.trim().length > 0 && !text.startsWith("//"));
  if (lines.length < 2) throw new Error("History must begin with 0b. and 0a. layout lines.");

  let position = parseInitialPosition(
    lines[0].text,
    lines[0].lineNumber,
    lines[1].text,
    lines[1].lineNumber,
  );
  try {
    engine.legalMoves(position);
  } catch (reason) {
    throw new Error(
      `Line ${lines[0].lineNumber}: invalid initial position${reason instanceof Error ? ` (${reason.message})` : ""}.`,
    );
  }
  const timeline: TimelineEntry[] = [{ position, move: null }];

  for (let lineIndex = 2; lineIndex < lines.length; lineIndex += 1) {
    const timelineIndex = lineIndex - 1;
    const expectedPrefix = formatPlyPrefix(timelineIndex, position.turn);
    const { text, lineNumber } = lines[lineIndex];
    if (!text.startsWith(`${expectedPrefix} `)) {
      throw new Error(`Line ${lineNumber}: expected prefix "${expectedPrefix}".`);
    }
    const body = text.slice(expectedPrefix.length + 1);
    const suffixMatch = body.match(/ ([+-]#0)$/);
    const assertedSuffix = suffixMatch?.[1] ?? "";
    const moveBody = suffixMatch ? body.slice(0, -suffixMatch[0].length) : body;
    const legalMoves = engine
      .legalMoves(position)
      .filter((move) => formatMove(position, move) === moveBody)
      .sort((left, right) => left.id.localeCompare(right.id));
    const move = legalMoves[0];
    if (!move) {
      throw new Error(`Line ${lineNumber}: "${body}" is not a legal move.`);
    }
    const previousPosition = position;
    try {
      position = engine.applyMove(position, move);
    } catch (reason) {
      throw new Error(
        `Line ${lineNumber}: move could not be applied${reason instanceof Error ? ` (${reason.message})` : ""}.`,
      );
    }
    const actualSuffix = snipeCaptureSuffix(previousPosition, position);
    if (assertedSuffix && assertedSuffix !== actualSuffix) {
      throw new Error(
        `Line ${lineNumber}: asserted result "${assertedSuffix}" does not match the actual snipe capture.`,
      );
    }
    timeline.push({ position, move });
  }
  return timeline;
}
