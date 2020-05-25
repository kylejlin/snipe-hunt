import { Card, CardType, GameState } from "./types";

export const gameUtil = { areCardsEqual, getRandomGameState };

function areCardsEqual(a: Card, b: Card): boolean {
  if (a.cardType === CardType.Snipe || b.cardType === CardType.Snipe) {
    return a.cardType === b.cardType && a.allegiance === b.allegiance;
  } else {
    return (
      a.cardType === b.cardType &&
      a.instance === b.instance &&
      a.allegiance === b.allegiance
    );
  }
}

function getRandomGameState(): GameState {}
