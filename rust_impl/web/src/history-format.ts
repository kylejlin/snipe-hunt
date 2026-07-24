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

export function formatPlyPrefix(timelineIndex: number, player: Player): string {
  if (timelineIndex < 1) throw new Error("Move plies begin at timeline index 1.");
  return `${Math.ceil(timelineIndex / 2)}${player === "Alpha" ? "a" : "b"}.`;
}

export function formatDisplayPlyPrefix(plyNumber: number, player: Player): string {
  if (plyNumber < 0) throw new Error("Ply numbers cannot be negative.");
  return `${plyNumber}${player === "Alpha" ? "α" : "β"}.`;
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
      `${formatPlyPrefix(index, entry.move.player)} ${formatMove(
        timeline[index - 1].position,
        entry.move,
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

function parseInitialPosition(betaLine: string, alphaLine: string): Position {
  const beta = parseLayoutLine(betaLine, 1, "Beta", "0b.");
  const alpha = parseLayoutLine(alphaLine, 2, "Alpha", "0a.");
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
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  while (lines.length > 0 && lines.at(-1)?.trim() === "") lines.pop();
  if (lines.length < 2) throw new Error("History must begin with 0b. and 0a. layout lines.");
  const blankLine = lines.findIndex((line) => line.trim().length === 0);
  if (blankLine >= 0) throw new Error(`Line ${blankLine + 1}: blank lines are not allowed.`);

  let position = parseInitialPosition(lines[0], lines[1]);
  try {
    engine.legalMoves(position);
  } catch (reason) {
    throw new Error(
      `Line 1: invalid initial position${reason instanceof Error ? ` (${reason.message})` : ""}.`,
    );
  }
  const timeline: TimelineEntry[] = [{ position, move: null }];

  for (let lineIndex = 2; lineIndex < lines.length; lineIndex += 1) {
    const timelineIndex = lineIndex - 1;
    const expectedPrefix = formatPlyPrefix(timelineIndex, position.turn);
    if (!lines[lineIndex].startsWith(`${expectedPrefix} `)) {
      throw new Error(`Line ${lineIndex + 1}: expected prefix "${expectedPrefix}".`);
    }
    const body = lines[lineIndex].slice(expectedPrefix.length + 1);
    const legalMoves = engine
      .legalMoves(position)
      .filter((move) => formatMove(position, move) === body)
      .sort((left, right) => left.id.localeCompare(right.id));
    const move = legalMoves[0];
    if (!move) {
      throw new Error(`Line ${lineIndex + 1}: "${body}" is not a legal move.`);
    }
    try {
      position = engine.applyMove(position, move);
    } catch (reason) {
      throw new Error(
        `Line ${lineIndex + 1}: move could not be applied${reason instanceof Error ? ` (${reason.message})` : ""}.`,
      );
    }
    timeline.push({ position, move });
  }
  return timeline;
}
