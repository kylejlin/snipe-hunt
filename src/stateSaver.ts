import { option, Option } from "rusty-ts";
import { AppState, StateSaver, STATE_VERSION } from "./types";

enum LocalStorageKeys {
  AppState = "AppState",
}

const stateSaverImpl: StateSaver<AppState> = {
  getState(): Option<AppState> {
    const stateStr = localStorage.getItem(LocalStorageKeys.AppState);
    if (stateStr === null) {
      return option.none();
    } else {
      const state = JSON.parse(stateStr);
      if (state.stateVersion === STATE_VERSION) {
        const convertedState: AppState = {
          ...state,
          selectedCard: option.fromVoidable(state.selectedCard),
          gameState: {
            ...state.gameState,
            pendingSubPly: option.fromVoidable(state.gameState.pendingSubPly),
          },
        };
        return option.some(convertedState);
      } else {
        return option.none();
      }
    }
  },
  setState(state: AppState): void {
    const stateStr = JSON.stringify(state, (_k, v) => {
      if (
        v !== null &&
        "object" === typeof v &&
        "function" === typeof v.unwrap
      ) {
        return v.unwrapOr(null);
      } else {
        return v;
      }
    });
    localStorage.setItem(LocalStorageKeys.AppState, stateStr);
  },
};

export default stateSaverImpl;
