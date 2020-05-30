import { option, Option } from "rusty-ts";
import { AppState, WorkerMessageType } from "../types";

export {};

declare const self: Worker;

let ai: Option<AppState["ai"]> = option.none();

self.addEventListener("message", (e) => {
  const data = e.data;
  if (
    "object" === typeof data &&
    data !== null &&
    data.messageType === WorkerMessageType.UpdateAi
  ) {
  }
});
