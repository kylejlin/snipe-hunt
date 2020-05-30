import { Filter, PlyTag } from "./bitwiseUtils";
import { cardProperties } from "./cardMaps";
import { gameStateFactory } from "./gameStateFactory";
import randInt from "./randInt";
import {
  AnimalType,
  Card,
  CardLocation,
  CardType,
  GameState,
  Player,
  Ply,
  PlyType,
  Row,
  SnipeType,
} from "./types";

export function getRandomGameState(): GameState {
  const { alpha, beta } = getShuffledDecks();
  return gameStateFactory.fromBoard({
    [CardLocation.AlphaReserve]: [alpha.pop()!],
    [CardLocation.Row1]: [
      alpha.pop()!,
      { cardType: CardType.AlphaSnipe, allegiance: Player.Alpha },
      alpha.pop()!,
    ],
    [CardLocation.Row2]: [
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
      alpha.pop()!,
    ],
    [CardLocation.Row3]: [alpha.pop()!],

    [CardLocation.Row4]: [beta.pop()!],
    [CardLocation.Row5]: [
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
      beta.pop()!,
    ],
    [CardLocation.Row6]: [
      beta.pop()!,
      { cardType: CardType.BetaSnipe, allegiance: Player.Beta },
      beta.pop()!,
    ],
    [CardLocation.BetaReserve]: [beta.pop()!],
  });
}

function getShuffledDecks(): { alpha: Card[]; beta: Card[] } {
  const minors: CardType[] = [
    CardType.Mouse1,
    CardType.Ox1,
    CardType.Rabbit1,
    CardType.Snake1,
    CardType.Horse1,
    CardType.Ram1,
    CardType.Monkey1,
    CardType.Rooster1,
    CardType.Dog1,
    CardType.Boar1,

    CardType.Squid1,
    CardType.Frog1,

    CardType.Mouse2,
    CardType.Ox2,
    CardType.Rabbit2,
    CardType.Snake2,
    CardType.Horse2,
    CardType.Ram2,
    CardType.Monkey2,
    CardType.Rooster2,
    CardType.Dog2,
    CardType.Boar2,

    CardType.Squid2,
    CardType.Frog2,
  ];
  const majors: CardType[] = [
    CardType.Tiger1,
    CardType.Dragon1,
    CardType.Fish1,
    CardType.Elephant1,

    CardType.Tiger2,
    CardType.Dragon2,
    CardType.Fish2,
    CardType.Elephant2,
  ];

  shuffle(minors);
  shuffle(majors);

  const alpha: Card[] = minors
    .slice(0, 12)
    .concat(majors.slice(0, 4))
    .map((cardType) => ({ cardType, allegiance: Player.Alpha }));
  const beta: Card[] = minors
    .slice(12)
    .concat(majors.slice(4))
    .map((cardType) => ({ cardType, allegiance: Player.Beta }));

  shuffle(alpha);
  shuffle(beta);

  return { alpha, beta };
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

export function isReserve(
  location: CardLocation
): location is CardLocation.AlphaReserve | CardLocation.BetaReserve {
  return (
    location === CardLocation.AlphaReserve ||
    location === CardLocation.BetaReserve
  );
}

export function canRetreat(cardType: CardType): boolean {
  return cardProperties[cardType].canRetreat;
}

export function opponentOf(player: Player): Player {
  switch (player) {
    case Player.Alpha:
      return Player.Beta;
    case Player.Beta:
      return Player.Alpha;
  }
}

export function snipeOf(player: Player): SnipeType {
  switch (player) {
    case Player.Alpha:
      return CardType.AlphaSnipe;
    case Player.Beta:
      return CardType.BetaSnipe;
  }
}

export function oneRowForward(row: Row, player: Player): CardLocation {
  const diff = player === Player.Alpha ? 1 : -1;
  return row + diff;
}

export function oneRowBackward(row: Row, player: Player): CardLocation {
  const diff = player === Player.Alpha ? -1 : 1;
  return row + diff;
}

export function isRow(location: CardLocation): location is Row {
  return !(
    location === CardLocation.AlphaReserve ||
    location === CardLocation.BetaReserve
  );
}

export function decodePly(ply: number): Ply {
  const tag = (ply & Filter.LeastThreeBits) as PlyTag;
  switch (tag) {
    case PlyTag.SnipeStep: {
      const destination = ((ply >>> 3) & Filter.LeastThreeBits) as Row;
      return { plyType: PlyType.SnipeStep, destination };
    }

    case PlyTag.Drop: {
      const cardType = ((ply >>> 3) & Filter.LeastFiveBits) as AnimalType;
      const destination = ((ply >>> 8) & Filter.LeastThreeBits) as Row;
      return {
        plyType: PlyType.Drop,
        dropped: cardType,
        destination,
      };
    }

    case PlyTag.TwoAnimalSteps: {
      const firstCardType = ((ply >>> 3) & Filter.LeastFiveBits) as AnimalType;
      const firstDestination = ((ply >>> 8) & Filter.LeastThreeBits) as Row;
      const secondCardType = ((ply >>> 11) &
        Filter.LeastFiveBits) as AnimalType;
      const secondDestination = ((ply >>> 16) & Filter.LeastThreeBits) as Row;
      return {
        plyType: PlyType.TwoAnimalSteps,
        first: { moved: firstCardType, destination: firstDestination },
        second: { moved: secondCardType, destination: secondDestination },
      };
    }
  }
}
