import React from "react";
import { option, Option, Result } from "rusty-ts";
import { getAnalyzer } from "./analyzer";
import "./App.css";
import { cardEmojis } from "./cardMaps";
import AnimalStepView from "./components/AnimalStepView";
import CardView from "./components/CardView";
import ElementMatrix from "./components/ElementMatrix";
import FutureAnimalStepView from "./components/FutureAnimalStepView";
import PlyView from "./components/PlyView";
import * as gameUtil from "./gameUtil";
import { futureSubPlyStackSaver, gameStateSaver } from "./stateSavers";
import {
  AnimalStep,
  AnimalType,
  AppState,
  Card,
  CardLocation,
  CardType,
  Drop,
  FutureSubPlyStack,
  GameState,
  IllegalGameStateUpdate,
  MctsWorkerMessage,
  Player,
  PlyType,
  Row,
  SnipeStep,
  STATE_VERSION,
  WorkerMessageType,
  MctsAnalysis,
  UpdateMctsAnalyzerGameStateRequest,
  Ply,
  Atomic,
} from "./types";
import MctsWorker from "./workers/mcts.importable";

export default class App extends React.Component<{}, AppState> {
  private mctsWorker: Worker | undefined;
  private hasMounted: boolean;

  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();

    (window as any).app = this;

    this.hasMounted = false;
  }

  componentDidMount(): void {
    const mctsWorker = new MctsWorker();
    mctsWorker.addEventListener("message", this.onMctsWorkerMessage);
    this.mctsWorker = mctsWorker;

    this.updateMctsAnalyzerGameState(this.state.gameState);

    this.hasMounted = true;
  }

  bindMethods(): void {
    this.onCardClicked = this.onCardClicked.bind(this);
    this.onResetClicked = this.onResetClicked.bind(this);
    this.onUndoSubPlyClicked = this.onUndoSubPlyClicked.bind(this);
    this.onRedoSubPlyClicked = this.onRedoSubPlyClicked.bind(this);
    this.onMctsWorkerMessage = this.onMctsWorkerMessage.bind(this);
    this.isMctsAnalysisUpToDate = this.isMctsAnalysisUpToDate.bind(this);
  }

  updateMctsAnalyzerGameState(gameState: GameState): void {
    if (this.mctsWorker === undefined) {
      throw new Error(
        "Cannot updateMctsAnalyzerGameState() because mctsAnalyzer has not been initialized yet."
      );
    }

    const message: UpdateMctsAnalyzerGameStateRequest = {
      messageType: WorkerMessageType.UpdateMctsAnalyzerGameStateRequest,
      gameState,
    };
    this.mctsWorker.postMessage(message);
  }

  saveAndUpdateGameState(newGameState: GameState) {
    gameStateSaver.setState(newGameState);
    this.setState({
      gameState: newGameState,
    });

    this.updateMctsAnalyzerGameState(newGameState);
  }

  render(): React.ReactElement {
    return this.renderMatrixView();
  }

  renderMatrixView(): React.ReactElement {
    const analyzer = getAnalyzer(this.state.gameState);
    const initialBoard = getAnalyzer(analyzer.getInitialState()).getBoard();
    const currentBoard = analyzer.getBoard();
    const plies = analyzer.getPlies();

    const { selectedCardType: selectedCard } = this.state.ux;

    return (
      <div className="SnipeHunt">
        {analyzer.getWinner().match({
          none: () => <div>Turn: {Player[analyzer.getTurn()]}</div>,
          some: (winner) => <div>Winner: {Player[winner]}</div>,
        })}

        <div>
          <table className="Board">
            <tbody>
              <tr>
                <td className="BoardCell">
                  Reserve
                  {this.renderSnipesIn(currentBoard[CardLocation.AlphaReserve])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.AlphaReserve]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={(e) => {
                    if (
                      e.target instanceof HTMLTableCellElement &&
                      e.target.classList.contains("BoardCell")
                    ) {
                      this.onRowNumberClicked(1);
                    }
                  }}
                >
                  1{this.renderSnipesIn(currentBoard[CardLocation.Row1])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.Row1]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={(e) => {
                    if (
                      e.target instanceof HTMLTableCellElement &&
                      e.target.classList.contains("BoardCell")
                    ) {
                      this.onRowNumberClicked(2);
                    }
                  }}
                >
                  2{this.renderSnipesIn(currentBoard[CardLocation.Row2])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.Row2]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={(e) => {
                    if (
                      e.target instanceof HTMLTableCellElement &&
                      e.target.classList.contains("BoardCell")
                    ) {
                      this.onRowNumberClicked(3);
                    }
                  }}
                >
                  3{this.renderSnipesIn(currentBoard[CardLocation.Row3])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.Row3]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={(e) => {
                    if (
                      e.target instanceof HTMLTableCellElement &&
                      e.target.classList.contains("BoardCell")
                    ) {
                      this.onRowNumberClicked(4);
                    }
                  }}
                >
                  4{this.renderSnipesIn(currentBoard[CardLocation.Row4])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.Row4]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={(e) => {
                    if (
                      e.target instanceof HTMLTableCellElement &&
                      e.target.classList.contains("BoardCell")
                    ) {
                      this.onRowNumberClicked(5);
                    }
                  }}
                >
                  5{this.renderSnipesIn(currentBoard[CardLocation.Row5])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.Row5]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={(e) => {
                    if (
                      e.target instanceof HTMLTableCellElement &&
                      e.target.classList.contains("BoardCell")
                    ) {
                      this.onRowNumberClicked(6);
                    }
                  }}
                >
                  6{this.renderSnipesIn(currentBoard[CardLocation.Row6])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.Row6]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td className="BoardCell">
                  Reserve
                  {this.renderSnipesIn(currentBoard[CardLocation.BetaReserve])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    analyzer={analyzer}
                    cards={currentBoard[CardLocation.BetaReserve]}
                    selectedCard={selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div>
          <h3>Plies</h3>
          <ol className="Plies">
            <li>
              <div className="PlyNumber">
                {cardEmojis[CardType.BetaSnipe] + "1"}.
              </div>{" "}
              =
              {initialBoard[CardLocation.BetaReserve].map(
                (card) => cardEmojis[card.cardType]
              )}
              ;{" "}
              {initialBoard[CardLocation.Row6].map(
                (card) => cardEmojis[card.cardType]
              )}
              ;{" "}
              {initialBoard[CardLocation.Row5].map(
                (card) => cardEmojis[card.cardType]
              )}
              ;{" "}
              {initialBoard[CardLocation.Row4].map(
                (card) => cardEmojis[card.cardType]
              )}
            </li>
            <li>
              <div className="PlyNumber">
                {cardEmojis[CardType.AlphaSnipe] + "2"}.
              </div>{" "}
              =
              {initialBoard[CardLocation.AlphaReserve].map(
                (card) => cardEmojis[card.cardType]
              )}
              ;{" "}
              {initialBoard[CardLocation.Row1].map(
                (card) => cardEmojis[card.cardType]
              )}
              ;{" "}
              {initialBoard[CardLocation.Row2].map(
                (card) => cardEmojis[card.cardType]
              )}
              ;{" "}
              {initialBoard[CardLocation.Row3].map(
                (card) => cardEmojis[card.cardType]
              )}
            </li>

            {plies.map((ply, zeroBasedPlyNumber) => {
              const plyNumber = zeroBasedPlyNumber + 3;
              return <PlyView ply={ply} plyNumber={plyNumber} />;
            })}

            {analyzer.getPendingAnimalStep().match({
              none: () => null,
              some: (step) => (
                <AnimalStepView
                  step={step}
                  plyNumber={plies.length + 3}
                  winner={analyzer.getWinner()}
                />
              ),
            })}

            {analyzer.getWinner().match({
              none: () => null,
              some: (winner) => {
                const winnerEmoji = cardEmojis[gameUtil.snipeOf(winner)];
                const loserEmoji =
                  cardEmojis[gameUtil.snipeOf(gameUtil.opponentOf(winner))];
                return winnerEmoji + ">" + loserEmoji;
              },
            })}
          </ol>
          <h4>Future sub plies</h4>
          <ol className="Plies">{this.renderFutureSubPlies()}</ol>
          <button onClick={this.onUndoSubPlyClicked}>Back</button>
          <button onClick={this.onRedoSubPlyClicked}>Forward</button>
        </div>
        <button onClick={this.onResetClicked}>Reset</button>
        <div>
          <h3>Computer agents:</h3>
          {this.state.mctsAnalysis.match({
            none: () =>
              analyzer.isGameOver() ? <p>Game over</p> : <p>Loading...</p>,
            some: (analysis) => {
              const { bestAtomic } = analysis;
              const afterPerformingBest = getAnalyzer(
                analyzer.forcePerform(bestAtomic)
              );
              const currentStateMeanValue =
                analysis.currentStateValue / analysis.currentStateRollouts;
              const bestAtomicMeanValue =
                analysis.bestAtomicValue / analysis.bestAtomicRollouts;

              return (
                <>
                  <h4>
                    MCTS (before: [v̅ = {currentStateMeanValue.toFixed(3)}, n ={" "}
                    {analysis.currentStateRollouts}
                    ], after: [v̅ = {bestAtomicMeanValue.toFixed(3)}, n ={" "}
                    {analysis.bestAtomicRollouts}
                    ]):
                  </h4>
                  {"plyType" in bestAtomic ? (
                    <PlyView ply={bestAtomic} plyNumber={plies.length + 3} />
                  ) : analyzer.getPendingAnimalStep().isSome() ? (
                    <FutureAnimalStepView
                      step={bestAtomic}
                      plyNumber={plies.length + 3}
                    />
                  ) : (
                    <AnimalStepView
                      step={bestAtomic}
                      plyNumber={plies.length + 3}
                      winner={afterPerformingBest.getWinner()}
                    />
                  )}
                </>
              );
            },
          })}
        </div>
      </div>
    );
  }

  renderSnipesIn(cards: Card[]): React.ReactElement[] {
    const selectedCardType = this.state.ux.selectedCardType.map((card) => card);
    const snipes = cards.filter(
      (card) =>
        card.cardType === CardType.AlphaSnipe ||
        card.cardType === CardType.BetaSnipe
    );
    return snipes.map((card) => {
      const isSelected = selectedCardType.equalsSome(card.cardType);
      return (
        <CardView
          key={card.cardType}
          card={card}
          isSelected={isSelected}
          onCardClicked={this.onCardClicked}
        />
      );
    });
  }

  renderTraditionalView(): React.ReactElement {
    const analyzer = getAnalyzer(this.state.gameState);
    const currentBoard = analyzer.getBoard();

    return (
      <table>
        <tbody>
          <tr>
            <td>Reserve</td>
            <td
              className={
                analyzer.isGameOver()
                  ? ""
                  : analyzer.getTurn() === Player.Alpha
                  ? "TurnIndicatorLight"
                  : ""
              }
            />
            <td>{this.renderCards(currentBoard[CardLocation.AlphaReserve])}</td>
          </tr>
          <tr>
            <td onClick={() => this.onRowNumberClicked(1)}>1</td>
            <td>
              {this.renderCards(currentBoard[CardLocation.Row1].filter(isBeta))}
            </td>
            <td>
              {this.renderCards(
                currentBoard[CardLocation.Row1].filter(isAlpha)
              )}
            </td>
          </tr>
          <tr>
            <td onClick={() => this.onRowNumberClicked(2)}>2</td>
            <td>
              {this.renderCards(currentBoard[CardLocation.Row2].filter(isBeta))}
            </td>
            <td>
              {this.renderCards(
                currentBoard[CardLocation.Row2].filter(isAlpha)
              )}
            </td>
          </tr>{" "}
          <tr>
            <td onClick={() => this.onRowNumberClicked(3)}>3</td>
            <td>
              {this.renderCards(currentBoard[CardLocation.Row3].filter(isBeta))}
            </td>
            <td>
              {this.renderCards(
                currentBoard[CardLocation.Row3].filter(isAlpha)
              )}
            </td>
          </tr>{" "}
          <tr>
            <td onClick={() => this.onRowNumberClicked(4)}>4</td>
            <td>
              {this.renderCards(currentBoard[CardLocation.Row4].filter(isBeta))}
            </td>
            <td>
              {this.renderCards(
                currentBoard[CardLocation.Row4].filter(isAlpha)
              )}
            </td>
          </tr>{" "}
          <tr>
            <td onClick={() => this.onRowNumberClicked(5)}>5</td>
            <td>
              {this.renderCards(currentBoard[CardLocation.Row5].filter(isBeta))}
            </td>
            <td>
              {this.renderCards(
                currentBoard[CardLocation.Row5].filter(isAlpha)
              )}
            </td>
          </tr>{" "}
          <tr>
            <td onClick={() => this.onRowNumberClicked(6)}>6</td>
            <td>
              {this.renderCards(currentBoard[CardLocation.Row6].filter(isBeta))}
            </td>
            <td>
              {this.renderCards(
                currentBoard[CardLocation.Row6].filter(isAlpha)
              )}
            </td>
          </tr>
          <tr>
            <td>Reserve</td>
            <td
              className={
                analyzer.isGameOver()
                  ? ""
                  : analyzer.getTurn() === Player.Beta
                  ? "TurnIndicatorLight"
                  : ""
              }
            />
            <td>{this.renderCards(currentBoard[CardLocation.BetaReserve])}</td>
          </tr>
        </tbody>
      </table>
    );
  }

  renderCards(cards: Card[]): React.ReactElement[] {
    return cards.map((card) => (
      <CardView
        key={card.cardType}
        card={card}
        isSelected={this.state.ux.selectedCardType.match({
          none: () => false,
          some: (selectedCardType) => selectedCardType === card.cardType,
        })}
        onCardClicked={() => this.onCardClicked(card)}
      />
    ));
  }

  renderFutureSubPlies() {
    const chronological = this.state.ux.futureSubPlyStack.atomics
      .slice()
      .reverse();
    let preTurnsAnimalStep: Option<AnimalStep> = option.none();
    const turns: Ply[] = [];
    let postTurnsAnimalStep: Option<AnimalStep> = option.none();

    let i = 0;
    while (i < chronological.length) {
      const next = chronological[i];

      if ("plyType" in next) {
        turns.push(next);
        i++;
      } else if (i + 1 < chronological.length) {
        i++;
        const nextNext = chronological[i];

        if ("plyType" in nextNext) {
          preTurnsAnimalStep = option.some(next);

          turns.push(nextNext);
          i++;
        } else {
          turns.push({
            plyType: PlyType.TwoAnimalSteps,
            first: next,
            second: nextNext,
          });
          i++;
        }
      } else {
        postTurnsAnimalStep = option.some(next);
        i++;
      }
    }

    const { plies } = this.state.gameState;
    const analyzer = getAnalyzer(this.state.gameState);

    return (
      <>
        {preTurnsAnimalStep.match({
          none: () => null,
          some: (step) => (
            <FutureAnimalStepView step={step} plyNumber={plies.length + 3} />
          ),
        })}

        {turns.map((ply, i) => (
          <PlyView
            ply={ply}
            plyNumber={
              plies.length +
              3 +
              preTurnsAnimalStep.match({ none: () => 0, some: () => 1 }) +
              i
            }
          />
        ))}

        {postTurnsAnimalStep.match({
          none: () => null,
          some: (step) => (
            <AnimalStepView
              step={step}
              plyNumber={
                plies.length +
                3 +
                preTurnsAnimalStep.match({ none: () => 0, some: () => 1 }) +
                turns.length
              }
              winner={getAnalyzer(analyzer.forcePerform(step)).getWinner()}
            />
          ),
        })}
      </>
    );
  }

  onCardClicked(clicked: Card): void {
    const analyzer = getAnalyzer(this.state.gameState);
    if (clicked.allegiance !== analyzer.getTurn()) {
      return;
    }

    const { selectedCardType } = this.state.ux;
    const isClickedCardAlreadySelected = selectedCardType.someSatisfies(
      (selectedType) => selectedType === clicked.cardType
    );
    if (isClickedCardAlreadySelected) {
      this.updateUxState({ selectedCardType: option.none() });
    } else {
      this.updateUxState({ selectedCardType: option.some(clicked.cardType) });
    }
  }

  updateUxState(newUxState: Partial<AppState["ux"]>): void {
    this.setState((prevState) => {
      const newState = {
        ...prevState,
        ux: {
          ...prevState.ux,
          ...newUxState,
        },
      };
      futureSubPlyStackSaver.setState(newState.ux.futureSubPlyStack);
      return newState;
    });
  }

  onRowNumberClicked(row: Row): void {
    const analyzer = getAnalyzer(this.state.gameState);

    if (analyzer.isGameOver()) {
      return;
    }

    const { selectedCardType: selectedCard } = this.state.ux;

    selectedCard.ifSome((selected) => {
      this.updateUxState({
        futureSubPlyStack: {
          stateVersion: STATE_VERSION,
          atomics: this.state.ux.futureSubPlyStack.atomics.slice(0, -1),
        },
      });

      const location = analyzer.getCardLocation(selected);
      if (gameUtil.isReserve(location)) {
        this.tryDrop(selected as AnimalType, row);
      } else {
        if (
          selected === CardType.AlphaSnipe ||
          selected === CardType.BetaSnipe
        ) {
          this.trySnipeStep(row);
        } else {
          this.tryAnimalStep(selected, row);
        }
      }
    });
  }

  tryDrop(selected: AnimalType, destination: Row): void {
    const analyzer = getAnalyzer(this.state.gameState);
    const drop: Drop = {
      plyType: PlyType.Drop,
      dropped: selected,
      destination,
    };

    const dropResult = analyzer.tryPerform(drop);
    this.updateGameStateOrAlertError(dropResult);
  }

  updateGameStateOrAlertError(
    res: Result<GameState, IllegalGameStateUpdate>
  ): void {
    res.match({
      ok: (newGameState) => {
        this.saveAndUpdateGameState(newGameState);
        this.updateUxState({ selectedCardType: option.none() });
      },
      err: (errorCode) => {
        alert(IllegalGameStateUpdate[errorCode]);
        this.updateUxState({ selectedCardType: option.none() });
      },
    });
  }

  trySnipeStep(destination: Row): void {
    const analyzer = getAnalyzer(this.state.gameState);
    const step: SnipeStep = {
      plyType: PlyType.SnipeStep,
      destination,
    };

    const stepResult = analyzer.tryPerform(step);
    this.updateGameStateOrAlertError(stepResult);
  }

  tryAnimalStep(selected: AnimalType, destination: Row): void {
    const analyzer = getAnalyzer(this.state.gameState);
    const step: AnimalStep = {
      moved: selected,
      destination,
    };

    const stepResult = analyzer.tryPerform(step);
    this.updateGameStateOrAlertError(stepResult);
  }

  onUndoSubPlyClicked(): void {
    const analyzer = getAnalyzer(this.state.gameState);

    const undoResult = analyzer.tryUndoSubPly();
    undoResult.match({
      ok: ({ newState: newGameState, undone }) => {
        this.updateUxState({
          futureSubPlyStack: this.getFutureSubPlyStackAfterUndoing(undone),
        });

        this.saveAndUpdateGameState(newGameState);
      },

      err: (errorCode) => {
        alert(IllegalGameStateUpdate[errorCode]);
      },
    });
  }

  getFutureSubPlyStackAfterUndoing(
    undone: Atomic
  ): AppState["ux"]["futureSubPlyStack"] {
    const stack = this.state.ux.futureSubPlyStack;

    return {
      stateVersion: stack.stateVersion,
      atomics: stack.atomics.concat([undone]),
    };
  }

  onRedoSubPlyClicked(): void {
    const { futureSubPlyStack } = this.state.ux;
    const { atomics } = futureSubPlyStack;

    if (atomics.length === 0) {
      alert("Nothing to redo.");
      return;
    }

    const nextAtomic = atomics[atomics.length - 1];
    const analyzer = getAnalyzer(this.state.gameState);
    const redoResult = analyzer.tryPerform(nextAtomic);

    this.updateGameStateOrAlertError(redoResult);

    redoResult.ifOk(() => {
      this.updateUxState({
        futureSubPlyStack: {
          stateVersion: futureSubPlyStack.stateVersion,
          atomics: atomics.slice(0, -1),
        },
      });
    });
  }

  onResetClicked(): void {
    if (window.confirm("Are you sure you want to reset?")) {
      const gameState = gameUtil.getRandomGameState();
      const state: AppState = {
        gameState,
        ux: {
          selectedCardType: option.none(),
          futureSubPlyStack: {
            stateVersion: STATE_VERSION,
            atomics: [],
          },
        },
        mctsAnalysis: option.none(),
      };
      gameStateSaver.setState(state.gameState);
      futureSubPlyStackSaver.setState(state.ux.futureSubPlyStack);
      this.setState(state);
    }
  }

  onMctsWorkerMessage(event: MessageEvent): void {
    const { data } = event;
    if ("object" === typeof data && data !== null) {
      const message: MctsWorkerMessage = data;
      switch (message.messageType) {
        case WorkerMessageType.LogNotification:
          console.log("MCTS Worker Log:", message.data);
          break;
        case WorkerMessageType.UpdateMctsAnalysisNotification:
          this.onMctsAnalysisUpdate(
            option
              .fromVoidable(message.optAnalysis)
              .filter(this.isMctsAnalysisUpToDate)
          );
          break;
      }
    }
  }

  onMctsAnalysisUpdate(optAnalysis: Option<MctsAnalysis>): void {
    if (this.hasMounted) {
      this.setState({ mctsAnalysis: optAnalysis });
    }
  }

  isMctsAnalysisUpToDate({ bestAtomic }: MctsAnalysis): boolean {
    return getAnalyzer(this.state.gameState).tryPerform(bestAtomic).isOk();
  }
}

function loadState(): AppState {
  const gameState = gameStateSaver.getState().unwrapOrElse(() => {
    const newGameState = gameUtil.getRandomGameState();
    gameStateSaver.setState(newGameState);
    return newGameState;
  });
  const futureSubPlyStack = futureSubPlyStackSaver
    .getState()
    .unwrapOrElse(() => {
      const newStack: FutureSubPlyStack = {
        stateVersion: STATE_VERSION,
        atomics: [],
      };
      futureSubPlyStackSaver.setState(newStack);
      return newStack;
    });

  return {
    gameState,
    ux: {
      selectedCardType: option.none(),
      futureSubPlyStack,
    },
    mctsAnalysis: option.none(),
  };
}

function isAlpha(card: Card): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: Card): boolean {
  return card.allegiance === Player.Beta;
}
