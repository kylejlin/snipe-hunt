import { gameStateImplUtils } from "./gameStateImpl";
import {
  AllegiantCard,
  Card,
  CardLocation,
  CardType,
  GameState,
  Player,
} from "./types";

export const gameUtil = {
  areCardsEqual,
  getRandomGameState,
  isReserve,
};

function areCardsEqual(a: Card, b: Card): boolean {
  return a.cardType === b.cardType && a.instance === b.instance;
}

function getRandomGameState(): GameState {
  const { alpha, beta } = getShuffledDecks();
  return gameStateImplUtils.fromBoard({
    [CardLocation.AlphaReserve]: [alpha.pop()!],
    [CardLocation.Row1]: [
      alpha.pop()!,
      { cardType: CardType.Snipe, instance: 0, allegiance: Player.Alpha },
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
      { cardType: CardType.Snipe, instance: 1, allegiance: Player.Beta },
      beta.pop()!,
    ],
    [CardLocation.BetaReserve]: [beta.pop()!],
  });
}

function getShuffledDecks(): { alpha: AllegiantCard[]; beta: AllegiantCard[] } {
  const minors: Card[] = [
    { cardType: CardType.Mouse, instance: 0 },
    { cardType: CardType.Ox, instance: 0 },
    { cardType: CardType.Rabbit, instance: 0 },
    { cardType: CardType.Snake, instance: 0 },
    { cardType: CardType.Horse, instance: 0 },
    { cardType: CardType.Ram, instance: 0 },
    { cardType: CardType.Monkey, instance: 0 },
    { cardType: CardType.Rooster, instance: 0 },
    { cardType: CardType.Dog, instance: 0 },
    { cardType: CardType.Boar, instance: 0 },
    { cardType: CardType.Squid, instance: 0 },
    { cardType: CardType.Frog, instance: 0 },

    { cardType: CardType.Mouse, instance: 1 },
    { cardType: CardType.Ox, instance: 1 },
    { cardType: CardType.Rabbit, instance: 1 },
    { cardType: CardType.Snake, instance: 1 },
    { cardType: CardType.Horse, instance: 1 },
    { cardType: CardType.Ram, instance: 1 },
    { cardType: CardType.Monkey, instance: 1 },
    { cardType: CardType.Rooster, instance: 1 },
    { cardType: CardType.Dog, instance: 1 },
    { cardType: CardType.Boar, instance: 1 },
    { cardType: CardType.Squid, instance: 1 },
    { cardType: CardType.Frog, instance: 1 },
  ];
  const majors: Card[] = [
    { cardType: CardType.Tiger, instance: 0 },
    { cardType: CardType.Dragon, instance: 0 },
    { cardType: CardType.Fish, instance: 0 },
    { cardType: CardType.Elephant, instance: 0 },

    { cardType: CardType.Tiger, instance: 1 },
    { cardType: CardType.Dragon, instance: 1 },
    { cardType: CardType.Fish, instance: 1 },
    { cardType: CardType.Elephant, instance: 1 },
  ];

  shuffle(minors);
  shuffle(majors);

  const alpha: AllegiantCard[] = minors
    .slice(0, 16)
    .concat(majors.slice(0, 4))
    .map((card) => ({ ...card, allegiance: Player.Alpha }));
  const beta: AllegiantCard[] = minors
    .slice(16)
    .concat(majors.slice(4))
    .map((card) => ({ ...card, allegiance: Player.Beta }));

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

function randInt(inclMin: number, exclMax: number): number {
  const diff = exclMax - inclMin;
  return inclMin + Math.floor(diff * Math.random());
}

function isReserve(location: CardLocation): boolean {
  return (
    location === CardLocation.AlphaReserve ||
    location === CardLocation.BetaReserve
  );
}
