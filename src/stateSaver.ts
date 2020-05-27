import { option, Option } from "rusty-ts";
import { gameAnalyzerUtils } from "./analyzer";
import { GameAnalyzer, StateSaver } from "./types";

enum LocalStorageKeys {
  AppState = "AppState",
}

const stateSaverImpl: StateSaver<GameAnalyzer> = {
  getState(): Option<GameAnalyzer> {
    const stateStr = localStorage.getItem(LocalStorageKeys.AppState);
    if (stateStr === null) {
      return option.none();
    } else {
      return gameAnalyzerUtils.fromString(stateStr);
    }
  },
  setState(state: GameAnalyzer): void {
    const stateStr = state.serialize();
    localStorage.setItem(LocalStorageKeys.AppState, stateStr);
  },
};

export default stateSaverImpl;
