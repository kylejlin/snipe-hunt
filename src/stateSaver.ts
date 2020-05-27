import { option, Option } from "rusty-ts";
import { getAnalyzer } from "./analyzer";
import { gameStateFactory } from "./gameStateFactory";
import { GameState, StateSaver } from "./types";

enum LocalStorageKeys {
  AppState = "AppState",
}

const stateSaverImpl: StateSaver<GameState> = {
  getState(): Option<GameState> {
    const stateStr = localStorage.getItem(LocalStorageKeys.AppState);
    if (stateStr === null) {
      return option.none();
    } else {
      return gameStateFactory.fromString(stateStr);
    }
  },
  setState(state: GameState): void {
    const stateStr = getAnalyzer(state).serialize();
    localStorage.setItem(LocalStorageKeys.AppState, stateStr);
  },
};

export default stateSaverImpl;
