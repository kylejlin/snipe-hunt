import { GameState, Board } from "./types";
import { Option } from "rusty-ts";

interface GameStateImpl extends GameState {}

interface GameStateStruct {}

export interface GameStateImplUtils {
  fromString(s: string): Option<GameStateImpl>;
  fromBoard(board: Board): GameStateImpl;
}

export const gameStateImplUtils = getGameStateImplUtils();

function getGameStateImplUtils(): GameStateImplUtils {
  return { fromString, fromBoard };

  function fromString(s: string): Option<GameStateImpl> {}

  function fromBoard(board: Board): GameStateImpl {}
}
