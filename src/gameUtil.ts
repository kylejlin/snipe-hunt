import { gameAnalyzerUtils } from "./analyzer";
import { Card, CardLocation, CardType, GameAnalyzer, Player } from "./types";
import { cardProperties } from "./cardMaps";

export function getRandomGameState(): GameAnalyzer {
  const { alpha, beta } = getShuffledDecks();
  return gameAnalyzerUtils.fromBoard({
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
    .slice(0, 16)
    .concat(majors.slice(0, 4))
    .map((cardType) => ({ cardType, allegiance: Player.Alpha }));
  const beta: Card[] = minors
    .slice(16)
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

function randInt(inclMin: number, exclMax: number): number {
  const diff = exclMax - inclMin;
  return inclMin + Math.floor(diff * Math.random());
}

export function isReserve(location: CardLocation): boolean {
  return (
    location === CardLocation.AlphaReserve ||
    location === CardLocation.BetaReserve
  );
}

export function canRetreat(cardType: CardType): boolean {
  return cardProperties[cardType].canRetreat;
}
