import { option, Option } from "rusty-ts";
import { getStateAnalyzer } from "./stateAnalyzer";
import { gameStateFactory } from "./gameStateFactory";
import {
  FutureSubPlyStack,
  GameState,
  StateSaver,
  STATE_VERSION,
} from "./types";

enum LocalStorageKeys {
  GameState = "GameState",
  FutureSubPlyStack = "FutureSubPlyStack",
  ThinkingTimeInMilliseconds = "ThinkingTimeInMilliseconds",
}

export const gameStateSaver: StateSaver<GameState> = {
  getState(): Option<GameState> {
    const stateStr = localStorage.getItem(LocalStorageKeys.GameState);
    if (stateStr === null) {
      return option.none();
    } else {
      return gameStateFactory.fromString(stateStr);
    }
  },

  setState(state: GameState): void {
    const stateStr = getStateAnalyzer(state).serialize();
    localStorage.setItem(LocalStorageKeys.GameState, stateStr);
  },
};

export const futureSubPlyStackSaver: StateSaver<FutureSubPlyStack> = {
  getState(): Option<FutureSubPlyStack> {
    const stateStr = localStorage.getItem(LocalStorageKeys.FutureSubPlyStack);
    if (stateStr === null) {
      return option.none();
    } else {
      const parsed = JSON.parse(stateStr);

      if (parsed.stateVersion !== STATE_VERSION) {
        return option.none();
      }

      parsed.pendingAnimalStep = option.fromVoidable(parsed.pendingAnimalStep);
      return option.some(parsed);
    }
  },

  setState(state: FutureSubPlyStack): void {
    const jsonified = {
      stateVersion: state.stateVersion,
      atomics: state.atomics,
    };
    const stateStr = JSON.stringify(jsonified);
    localStorage.setItem(LocalStorageKeys.FutureSubPlyStack, stateStr);
  },
};

export const thinkingTimeSaver: StateSaver<Option<number>> = {
  getState(): Option<Option<number>> {
    const stateStr = localStorage.getItem(
      LocalStorageKeys.ThinkingTimeInMilliseconds
    );
    if (stateStr === null) {
      return option.none();
    } else {
      const parsed: number | null = JSON.parse(stateStr);
      const optThinkingTimeInMS = option.fromVoidable(parsed);
      return option.some(optThinkingTimeInMS);
    }
  },

  setState(state: Option<number>): void {
    const jsonified = JSON.stringify(state.unwrapOr(null));
    localStorage.setItem(
      LocalStorageKeys.ThinkingTimeInMilliseconds,
      jsonified
    );
  },
};
