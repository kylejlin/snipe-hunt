import React from "react";
import ReactDOM from "react-dom";
import "./index.css";
import App from "./App";
import * as serviceWorker from "./serviceWorker";

import * as analyzer from "./analyzer";
import * as mcts from "./mcts";
import * as gameUtil from "./gameUtil";
import { CardType, MctsWorkerMessageType } from "./types";

(window as any).analyzer = analyzer;
(window as any).mcts = mcts;
(window as any).gameUtil = gameUtil;
(window as any).CardType = CardType;
(window as any).WorkerMessageType = MctsWorkerMessageType;

ReactDOM.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
  document.getElementById("root")
);

// If you want your app to work offline and load faster, you can change
// unregister() to register() below. Note this comes with some pitfalls.
// Learn more about service workers: https://bit.ly/CRA-PWA
serviceWorker.unregister();
