import { Option } from "rusty-ts";
import { Offset } from "./bitwiseUtils";
import { allCardLocations, Board, CardType, GameState, Player } from "./types";

export interface GameStateFactory {
  fromString(s: string): Option<GameState>;
  fromBoard(board: Board): GameState;
}

export const gameStateFactory: GameStateFactory = { fromString, fromBoard };

function fromString(s: string): Option<GameState> {}

function fromBoard(board: Board): GameState {
  const initialBoard = new Int32Array(24);

  for (const location of allCardLocations) {
    const cards = board[location];
    for (const card of cards) {
      if (
        card.cardType === CardType.AlphaSnipe ||
        card.cardType === CardType.BetaSnipe
      ) {
        const cardSet = card.cardType === CardType.AlphaSnipe ? 1 : 1 << 1;
        const offset = Offset.Snipes;
        initialBoard[location + offset] |= cardSet;
      } else {
        const cardSet = 1 << card.cardType;
        const offset =
          card.allegiance === Player.Alpha
            ? Offset.AlphaAnimals
            : Offset.BetaAnimals;
        initialBoard[location + offset] |= cardSet;
      }
    }
  }

  const data: GameState = {
    initialBoard,
    currentBoard: initialBoard,
    turn: Player.Beta,
    plies: [],
    pendingAnimalStep: 0,
  };

  return data;
}
