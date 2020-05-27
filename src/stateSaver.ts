import { option, Option } from "rusty-ts";
import { gameStateUtils, getAnalyzer } from "./analyzer";
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
      return gameStateUtils.fromString(stateStr);
    }
  },
  setState(state: GameState): void {
    const stateStr = getAnalyzer(state).serialize();
    localStorage.setItem(LocalStorageKeys.AppState, stateStr);
  },
};

export default stateSaverImpl;
