import { Card, CardType, GameState, AnimalStep, CardLocation } from "./types";
import { GameStateStruct, getGameState } from "./gameStateStruct";
import { Result } from "rusty-ts";

export const gameUtil = {
  areCardsEqual,
  getRandomGameState,
  isReserve,
};

function areCardsEqual(a: Card, b: Card): boolean {
  return a.cardType === b.cardType && a.instance === b.instance;
}

function getRandomGameState(): GameState {
  const deck = getShuffledAnimals();
  const struct: GameStateStruct = {};
  return getGameState(struct);
}

function getShuffledAnimals(): Omit<Card, "allegiance">[] {
  const animals = [
    { cardType: CardType.Mouse, instance: 0 },
    { cardType: CardType.Ox, instance: 0 },
    { cardType: CardType.Tiger, instance: 0 },
    { cardType: CardType.Rabbit, instance: 0 },
    { cardType: CardType.Dragon, instance: 0 },
    { cardType: CardType.Snake, instance: 0 },
    { cardType: CardType.Horse, instance: 0 },
    { cardType: CardType.Ram, instance: 0 },
    { cardType: CardType.Monkey, instance: 0 },
    { cardType: CardType.Rooster, instance: 0 },
    { cardType: CardType.Dog, instance: 0 },
    { cardType: CardType.Boar, instance: 0 },
    { cardType: CardType.Fish, instance: 0 },
    { cardType: CardType.Elephant, instance: 0 },
    { cardType: CardType.Squid, instance: 0 },
    { cardType: CardType.Frog, instance: 0 },

    { cardType: CardType.Mouse, instance: 1 },
    { cardType: CardType.Ox, instance: 1 },
    { cardType: CardType.Tiger, instance: 1 },
    { cardType: CardType.Rabbit, instance: 1 },
    { cardType: CardType.Dragon, instance: 1 },
    { cardType: CardType.Snake, instance: 1 },
    { cardType: CardType.Horse, instance: 1 },
    { cardType: CardType.Ram, instance: 1 },
    { cardType: CardType.Monkey, instance: 1 },
    { cardType: CardType.Rooster, instance: 1 },
    { cardType: CardType.Dog, instance: 1 },
    { cardType: CardType.Boar, instance: 1 },
    { cardType: CardType.Fish, instance: 1 },
    { cardType: CardType.Elephant, instance: 1 },
    { cardType: CardType.Squid, instance: 1 },
    { cardType: CardType.Frog, instance: 1 },
  ];
  shuffle(animals);
  return animals;
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

function isReserve(location: CardLocation): boolean {
  return (
    location === CardLocation.AlphaReserve ||
    location === CardLocation.BetaReserve
  );
}
