import { MctsWorkerMessage } from "../types";

export default class MctsWorker {
  postMessage(data: MctsWorkerMessage): void;
  postMessage(data: MctsWorkerMessage, transfer: Transferable[]): void;

  addEventListener: Worker["addEventListener"];
}
