import { GameState } from "./types";

export interface GameStateStruct {
  todo: "todo";
}

export function getGameState(struct: GameStateStruct): GameState {}
