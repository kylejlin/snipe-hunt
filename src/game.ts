import { option, Option, Result, result } from "rusty-ts";
import {
  AppState,
  Card,
  CardType,
  Element,
  GameState,
  Player,
  Position,
  Row,
  STATE_VERSION,
  CardPropertyMap,
  SubPlyType,
} from "./types";

export enum IllegalMove {
  NotYourCard,
  AlreadyMoved,
  AttackerInReserve,
  TargetInReserve,
  CannotEmptyRow,
  DestinationOutOfRange,
  InsufficientElements,
}

interface ElementCountTable {
  [Element.Fire]: [boolean, boolean, boolean];
  [Element.Water]: [boolean, boolean, boolean];
  [Element.Earth]: [boolean, boolean, boolean];
  [Element.Air]: [boolean, boolean, boolean];
}

const cardProperties: CardPropertyMap = {
  [CardType.AlphaSnipe]: {
    elements: option.none(),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.BetaSnipe]: {
    elements: option.none(),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },

  [CardType.Mouse]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Earth },
      promoted: { double: Element.Earth, single: Element.Water },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Ox]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Water },
      promoted: { double: Element.Air, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Tiger]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Fire },
      promoted: { double: Element.Fire, single: Element.Air },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Rabbit]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Water },
      promoted: { double: Element.Water, single: Element.Water },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Dragon]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Air },
      promoted: { double: Element.Air, single: Element.Water },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Snake]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Earth },
      promoted: { double: Element.Water, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Horse]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Air },
      promoted: { double: Element.Air, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Ram]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Air },
      promoted: { double: Element.Earth, single: Element.Air },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Monkey]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Earth },
      promoted: { double: Element.Earth, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Rooster]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Fire },
      promoted: { double: Element.Fire, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Dog]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Water },
      promoted: { double: Element.Air, single: Element.Air },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Boar]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Fire },
      promoted: { double: Element.Fire, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },

  [CardType.Fish]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Water },
      promoted: { double: Element.Water, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Elephant]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Earth },
      promoted: { double: Element.Fire, single: Element.Water },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Squid]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Fire },
      promoted: { double: Element.Earth, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Frog]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Air },
      promoted: { double: Element.Water, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
};

export function getRandomState(): AppState {
  const cards = getShuffledDeck();

  const alpha: Position = {
    reserve: [cards.pop()!].map(toUnpromotedAlpha),
    backRow: [
      CardType.AlphaSnipe,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
    ].map(toUnpromotedAlpha),
    frontRow: [cards.pop()!].map(toUnpromotedAlpha),
  };
  const beta: Position = {
    reserve: [cards.pop()!].map(toUnpromotedBeta),
    backRow: [
      CardType.BetaSnipe,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
      cards.pop()!,
    ].map(toUnpromotedBeta),
    frontRow: [cards.pop()!].map(toUnpromotedBeta),
  };

  return {
    stateVersion: STATE_VERSION,
    gameState: {
      turn: Player.Beta,
      alpha,
      beta,
      initialPositions: { alpha, beta },
      plies: [],
      futurePlies: [],
      pendingSubPly: option.none(),
    },
    selectedCard: option.none(),
  };
}

function getShuffledDeck(): CardType[] {
  const cards: CardType[] = [
    CardType.Mouse,
    CardType.Ox,
    CardType.Tiger,
    CardType.Rabbit,
    CardType.Dragon,
    CardType.Snake,
    CardType.Horse,
    CardType.Ram,
    CardType.Monkey,
    CardType.Rooster,
    CardType.Dog,
    CardType.Boar,
    CardType.Fish,
    CardType.Elephant,
    CardType.Squid,
    CardType.Frog,
  ];
  shuffle(cards);
  return cards;
}

function shuffle(arr: unknown[]): void {
  const SHUFFLE_REPETITIONS = 512;
  const len = arr.length;

  for (let k = 0; k < SHUFFLE_REPETITIONS; k++) {
    for (let i = 0; i <= len - 2; i++) {
      const j = randInt(i, len);
      const temp = arr[i];
      arr[i] = arr[j];
      arr[j] = temp;
    }
  }
}

function randInt(inclMin: number, exclMax: number): number {
  const range = exclMax - inclMin;
  return inclMin + Math.floor(Math.random() * range);
}

function toUnpromotedAlpha(cardType: CardType): Card {
  return { cardType, allegiance: Player.Alpha, isPromoted: false };
}

function toUnpromotedBeta(cardType: CardType): Card {
  return { cardType, allegiance: Player.Beta, isPromoted: false };
}

export function tryCapture(
  state: GameState,
  attackerType: CardType,
  targetType: CardType
): Result<GameState, IllegalMove> {
  const attacker = getCard(state, attackerType);
  const target = getCard(state, targetType);

  if (attacker.allegiance !== state.turn) {
    return result.err(IllegalMove.NotYourCard);
  }

  const alreadyMoved = state.pendingSubPly.match({
    none: () => false,
    some: (sub) => sub.subPlyType === SubPlyType.Move,
  });

  if (alreadyMoved) {
    return result.err(IllegalMove.AlreadyMoved);
  }

  const optAttackerRow = getRow(state, attackerType);
  const optTargetRow = getRow(state, targetType);

  if (optAttackerRow.isNone()) {
    return result.err(IllegalMove.AttackerInReserve);
  }
  if (optTargetRow.isNone()) {
    return result.err(IllegalMove.TargetInReserve);
  }

  const attackerRow = optAttackerRow.unwrap();
  const targetRow = optTargetRow.unwrap();

  if (getMutRow(state, attackerRow).length === 1) {
    return result.err(IllegalMove.CannotEmptyRow);
  }

  if (
    !(
      attackerRow + forward(attacker) === targetRow ||
      (canMoveBackward(attacker) &&
        attackerRow + backward(attacker) === targetRow)
    )
  ) {
    return result.err(IllegalMove.DestinationOutOfRange);
  }

  if (!doAttackerElementsTrumpTargetElements(attacker, target)) {
    return result.err(IllegalMove.InsufficientElements);
  }

  const newState = cloneGameState(state);
  let capturedCards: Card[];

  const mutAttackerRow = getMutRow(newState, attackerRow);
  mutAttackerRow.splice(
    mutAttackerRow.findIndex((c) => c.cardType === attacker.cardType),
    1
  );

  const mutTargetRow = getMutRow(newState, targetRow);
  if (hasTriplet(mutTargetRow.concat([attacker]))) {
    capturedCards = mutTargetRow.map(demoteIfNeeded);

    mutTargetRow.splice(0, mutTargetRow.length);
  } else {
    capturedCards = [demoteIfNeeded(target)];
    mutTargetRow.splice(
      mutTargetRow.findIndex((c) => c.cardType === target.cardType),
      1
    );
  }

  newState[getPlayerKey(attacker.allegiance)].reserve.push(...capturedCards);
  mutTargetRow.push(attacker);
  newState.pendingSubPly = option.some({
    subPlyType: SubPlyType.Move,
    moved: attacker.cardType,
    destination: targetRow,
    captures: capturedCards.map((c) => c.cardType),
  });

  return result.ok(newState);
}

function getRow(state: GameState, cardType: CardType): Option<Row> {
  if (state.alpha.backRow.some((card) => card.cardType === cardType)) {
    return option.some(1);
  }
  if (state.alpha.frontRow.some((card) => card.cardType === cardType)) {
    return option.some(2);
  }
  if (state.beta.frontRow.some((card) => card.cardType === cardType)) {
    return option.some(3);
  }
  if (state.beta.backRow.some((card) => card.cardType === cardType)) {
    return option.some(4);
  }
  return option.none();
}

function getCard(state: GameState, cardType: CardType): Card {
  function matchesType(card: Card): boolean {
    return card.cardType === cardType;
  }

  return (state.alpha.reserve.find(matchesType) ||
    state.alpha.backRow.find(matchesType) ||
    state.alpha.frontRow.find(matchesType) ||
    state.beta.frontRow.find(matchesType) ||
    state.beta.backRow.find(matchesType) ||
    state.beta.reserve.find(matchesType))!;
}

function forward(card: Card): number {
  if (card.allegiance === Player.Alpha) {
    return 1;
  } else {
    return -1;
  }
}

function backward(card: Card): number {
  return -forward(card);
}

function canMoveBackward(card: Card): boolean {
  const props = cardProperties[card.cardType];

  if (card.isPromoted) {
    return props.canPromotedMoveBackward;
  } else {
    return props.canUnpromotedMoveBackward;
  }
}

function doAttackerElementsTrumpTargetElements(
  attacker: Card,
  target: Card
): boolean {
  return option.all([getElements(attacker), getElements(target)]).match({
    none: () => false,
    some: ([attackerElements, targetElements]) =>
      doesTrump(attackerElements.double, targetElements.double) &&
      doesTrump(attackerElements.single, targetElements.single),
  });
}

function getElements(card: Card): Option<{ double: Element; single: Element }> {
  return cardProperties[card.cardType].elements.map((elements) =>
    card.isPromoted ? elements.promoted : elements.unpromoted
  );
}

function doesTrump(a: Element, b: Element): boolean {
  return (
    (a === Element.Fire && b === Element.Earth) ||
    (a === Element.Fire && b === Element.Air) ||
    (a === Element.Water && b === Element.Fire) ||
    (a === Element.Air && b === Element.Water) ||
    (a === Element.Air && b === Element.Earth)
  );
}

function cloneGameState(state: GameState): GameState {
  const str = JSON.stringify(state, (_k, v) => {
    if (v !== null && "object" === typeof v && "function" === typeof v.unwrap) {
      return v.unwrapOr(null);
    } else {
      return v;
    }
  });
  const parsed = JSON.parse(str);
  return {
    ...parsed,
    pendingSubPly: option.fromVoidable(parsed.pendingSubPly),
  };
}

function getMutRow(state: GameState, row: Row): Card[] {
  return [
    state.alpha.backRow,
    state.alpha.frontRow,
    state.beta.frontRow,
    state.beta.backRow,
  ][row - 1];
}

function hasTriplet(cards: Card[]): boolean {
  const table: ElementCountTable = {
    [Element.Fire]: [false, false, false],
    [Element.Water]: [false, false, false],
    [Element.Earth]: [false, false, false],
    [Element.Air]: [false, false, false],
  };

  for (const card of cards) {
    getElements(card).ifSome((elements) => {
      if (elements.double === elements.single) {
        table[elements.double][2] = true;
      } else {
        table[elements.double][1] = true;
        table[elements.single][0] = true;
      }
    });
  }

  return (
    (table[Element.Fire][0] &&
      table[Element.Fire][1] &&
      table[Element.Fire][2]) ||
    (table[Element.Water][0] &&
      table[Element.Water][1] &&
      table[Element.Water][2]) ||
    (table[Element.Earth][0] &&
      table[Element.Earth][1] &&
      table[Element.Earth][2]) ||
    (table[Element.Air][0] && table[Element.Air][1] && table[Element.Air][2])
  );
}

function getPlayerKey(player: Player): "alpha" | "beta" {
  switch (player) {
    case Player.Alpha:
      return "alpha";
    case Player.Beta:
      return "beta";
  }
}

function demoteIfNeeded(card: Card): Card {
  return { ...card, isPromoted: false };
}

export function tryMove(
  state: GameState,
  attackerType: CardType,
  destinationRow: Row
): Result<GameState, IllegalMove> {
  const attacker = getCard(state, attackerType);

  if (attacker.allegiance !== state.turn) {
    return result.err(IllegalMove.NotYourCard);
  }

  const alreadyMoved = state.pendingSubPly.match({
    none: () => false,
    some: (sub) => sub.subPlyType === SubPlyType.Move,
  });

  if (alreadyMoved) {
    return result.err(IllegalMove.AlreadyMoved);
  }

  const optAttackerRow = getRow(state, attackerType);

  if (optAttackerRow.isNone()) {
    return result.err(IllegalMove.AttackerInReserve);
  }

  const attackerRow = optAttackerRow.unwrap();

  if (getMutRow(state, attackerRow).length === 1) {
    return result.err(IllegalMove.CannotEmptyRow);
  }

  if (
    !(
      attackerRow + forward(attacker) === destinationRow ||
      (canMoveBackward(attacker) &&
        attackerRow + backward(attacker) === destinationRow)
    )
  ) {
    return result.err(IllegalMove.DestinationOutOfRange);
  }

  const newState = cloneGameState(state);
  let capturedCards: Card[];

  const mutAttackerRow = getMutRow(newState, attackerRow);
  console.log(mutAttackerRow, newState, attackerRow);
  mutAttackerRow.splice(
    mutAttackerRow.findIndex((c) => c.cardType === attacker.cardType),
    1
  );

  const mutTargetRow = getMutRow(newState, destinationRow);
  if (hasTriplet(mutTargetRow.concat([attacker]))) {
    capturedCards = mutTargetRow.map(demoteIfNeeded);

    mutTargetRow.splice(0, mutTargetRow.length);
  } else {
    capturedCards = [];
  }

  newState[getPlayerKey(attacker.allegiance)].reserve.push(...capturedCards);
  mutTargetRow.push(attacker);
  newState.pendingSubPly = option.some({
    subPlyType: SubPlyType.Move,
    moved: attacker.cardType,
    destination: destinationRow,
    captures: capturedCards.map((c) => c.cardType),
  });

  return result.ok(newState);
}
