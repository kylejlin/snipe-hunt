import React from "react";
import { option, Option, Result } from "rusty-ts";
import "./App.css";
import { cardEmojis } from "./cardMaps";
import AnalysisNode from "./components/AnalysisNode";
import AnimalStepView from "./components/AnimalStepView";
import CardView from "./components/CardView";
import ElementMatrix from "./components/ElementMatrix";
import FutureAnimalStepView from "./components/FutureAnimalStepView";
import PlyView from "./components/PlyView";
import * as gameUtil from "./gameUtil";
import { MctsAnalyzer, NodePointer, pointerToIndex } from "./mcts";
import { getMctsService } from "./mctsService";
import { getStateAnalyzer } from "./stateAnalyzer";
import {
  futureSubPlyStackSaver,
  gameStateSaver,
  thinkingTimeSaver,
} from "./stateSavers";
import {
  AnimalStep,
  AnimalType,
  AppState,
  Atomic,
  Card,
  CardLocation,
  CardType,
  Drop,
  FutureSubPlyStack,
  GameState,
  IllegalGameStateUpdate,
  MctsAnalysisSnapshot,
  MctsPausedState,
  MctsService,
  Player,
  Ply,
  PlyType,
  Row,
  SnipeStep,
  STATE_VERSION,
  SuggestionDetailLevel,
} from "./types";

const DEFAULT_THINKING_TIME_IN_MS = 90 * 1e3;

export default class App extends React.Component<{}, AppState> {
  private mctsService: MctsService;

  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();

    (window as any).app = this;

    this.mctsService = getMctsService();
  }

  componentDidMount(): void {
    this.mctsService.updateGameState(
      this.state.gameState,
      this.state.thinkingTimeInMS
    );

    this.mctsService.onSnapshot(this.onMctsServiceSnapshot);
    this.mctsService.onPause(this.onMctsServicePause);
    this.mctsService.onStopTimeChange(this.onMctsServiceOnStopTimeChange);
  }

  bindMethods(): void {
    this.onCardClicked = this.onCardClicked.bind(this);
    this.onResetGameClicked = this.onResetGameClicked.bind(this);
    this.onUndoSubPlyClicked = this.onUndoSubPlyClicked.bind(this);
    this.onRedoSubPlyClicked = this.onRedoSubPlyClicked.bind(this);
    this.isBestAtomicLegal = this.isBestAtomicLegal.bind(this);
    this.onMctsServiceSnapshot = this.onMctsServiceSnapshot.bind(this);
    this.onMctsServicePause = this.onMctsServicePause.bind(this);
    this.onPauseAnalyzerClicked = this.onPauseAnalyzerClicked.bind(this);
    this.onResumeAnalyzerClicked = this.onResumeAnalyzerClicked.bind(this);
    this.onDetailLevelChange = this.onDetailLevelChange.bind(this);
    this.onTimeLimitEnabledChange = this.onTimeLimitEnabledChange.bind(this);
    this.onThinkingTimeInputChange = this.onThinkingTimeInputChange.bind(this);
    this.onMctsServiceOnStopTimeChange = this.onMctsServiceOnStopTimeChange.bind(
      this
    );
  }

  saveAndUpdateGameState(newGameState: GameState) {
    gameStateSaver.setState(newGameState);
    this.setState({
      gameState: newGameState,
    });

    this.mctsService.updateGameState(newGameState, this.state.thinkingTimeInMS);
  }

  render(): React.ReactElement {
    return this.renderMatrixView();
  }

  renderMatrixView(): React.ReactElement {
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    const initialBoard = getStateAnalyzer(
      stateAnalyzer.getInitialState()
    ).getBoard();
    const currentBoard = stateAnalyzer.getBoard();
    const plies = stateAnalyzer.getPlies();
    const { mctsState } = this.state;

    const { selectedCardType: selectedCard } = this.state.ux;

    return (
      <div className="SnipeHunt">
        {stateAnalyzer.getWinner().match({
          none: () => <div>Turn: {Player[stateAnalyzer.getTurn()]}</div>,
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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
                    analyzer={stateAnalyzer}
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

            {stateAnalyzer.getPendingAnimalStep().match({
              none: () => null,
              some: (step) => (
                <AnimalStepView
                  step={step}
                  plyNumber={plies.length + 3}
                  winner={stateAnalyzer.getWinner()}
                />
              ),
            })}

            {stateAnalyzer.getWinner().match({
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
        <button onClick={this.onResetGameClicked}>Reset</button>
        <div>
          <h3>
            MCTS{" "}
            {mctsState.isRunning ? (
              <button onClick={this.onPauseAnalyzerClicked}>Pause</button>
            ) : (
              <button onClick={this.onResumeAnalyzerClicked}>Resume</button>
            )}
          </h3>

          <div>
            {this.state.thinkingTimeInMS.match({
              none: () => (
                <div>
                  <h4 className="Inline">Time limit</h4>{" "}
                  <input
                    type="checkbox"
                    checked={false}
                    onChange={this.onTimeLimitEnabledChange}
                  />
                </div>
              ),

              some: (thinkingTimeInMS) => (
                <>
                  <div>
                    <label>
                      <h4 className="Inline">Time limit</h4>{" "}
                      <input
                        type="checkbox"
                        checked={true}
                        onChange={this.onTimeLimitEnabledChange}
                      />
                    </label>
                  </div>

                  <div>
                    <label>
                      Seconds per turn:{" "}
                      <input
                        type="text"
                        value={this.state.thinkingTimeInputValue}
                        onChange={this.onThinkingTimeInputChange}
                      />
                    </label>
                  </div>

                  <div>
                    <label>
                      Seconds remaining for this turn:{" "}
                      <span>
                        {this.state.stopTime.match({
                          none: () => "Loading...",
                          some: (stopTime) => {
                            const diff = stopTime - Date.now();
                            const diffInSeconds = Math.ceil(diff * 1e-3);
                            const clamped = Math.max(0, diffInSeconds);
                            return "" + clamped;
                          },
                        })}
                      </span>
                    </label>
                  </div>
                </>
              ),
            })}
          </div>

          {mctsState.isRunning
            ? mctsState.mostRecentSnapshot.match({
                none: () =>
                  stateAnalyzer.isGameOver() ? (
                    <p>Game over</p>
                  ) : (
                    <p>Loading...</p>
                  ),
                some: (analysis) => {
                  const { bestAtomic } = analysis;
                  const afterPerformingBest = getStateAnalyzer(
                    stateAnalyzer.forcePerform(bestAtomic)
                  );
                  const currentStateMeanValue =
                    analysis.currentStateValue / analysis.currentStateRollouts;
                  const bestAtomicMeanValue =
                    analysis.bestAtomicValue / analysis.bestAtomicRollouts;

                  return (
                    <>
                      <h4>
                        Best action (before: [v̅ ={" "}
                        {currentStateMeanValue.toFixed(3)}, n ={" "}
                        {analysis.currentStateRollouts}
                        ], after: [v̅ = {bestAtomicMeanValue.toFixed(3)}, n ={" "}
                        {analysis.bestAtomicRollouts}
                        ]):
                      </h4>
                      {"plyType" in bestAtomic ? (
                        <PlyView
                          ply={bestAtomic}
                          plyNumber={plies.length + 3}
                        />
                      ) : stateAnalyzer.getPendingAnimalStep().isSome() ? (
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
              })
            : this.renderExpandableAnalysis(mctsState, plies.length + 2)}
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
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    const currentBoard = stateAnalyzer.getBoard();

    return (
      <table>
        <tbody>
          <tr>
            <td>Reserve</td>
            <td
              className={
                stateAnalyzer.isGameOver()
                  ? ""
                  : stateAnalyzer.getTurn() === Player.Alpha
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
                stateAnalyzer.isGameOver()
                  ? ""
                  : stateAnalyzer.getTurn() === Player.Beta
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
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);

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
              winner={getStateAnalyzer(
                stateAnalyzer.forcePerform(step)
              ).getWinner()}
            />
          ),
        })}
      </>
    );
  }

  renderExpandableAnalysis(
    mctsState: MctsPausedState,
    plyNumber: number
  ): React.ReactElement {
    const isTherePendingAnimalStep = getStateAnalyzer(this.state.gameState)
      .getPendingAnimalStep()
      .isSome();

    const mctsAnalyzer = mctsState.analyzer;
    const rootSummary = mctsAnalyzer.getNodeSummary(
      mctsAnalyzer.getRootPointer()
    );

    return (
      <AnalysisNode
        analyzer={mctsAnalyzer}
        suggestionDetailLevels={this.state.ux.analysisSuggestionDetailLevels}
        plyNumber={plyNumber}
        isTherePendingAnimalStep={isTherePendingAnimalStep}
        viewedNode={rootSummary}
        onDetailLevelChange={this.onDetailLevelChange}
      />
    );
  }

  onCardClicked(clicked: Card): void {
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    if (clicked.allegiance !== stateAnalyzer.getTurn()) {
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
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);

    if (stateAnalyzer.isGameOver()) {
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

      const location = stateAnalyzer.getCardLocation(selected);
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
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    const drop: Drop = {
      plyType: PlyType.Drop,
      dropped: selected,
      destination,
    };

    const dropResult = stateAnalyzer.tryPerform(drop);
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
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    const step: SnipeStep = {
      plyType: PlyType.SnipeStep,
      destination,
    };

    const stepResult = stateAnalyzer.tryPerform(step);
    this.updateGameStateOrAlertError(stepResult);
  }

  tryAnimalStep(selected: AnimalType, destination: Row): void {
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    const step: AnimalStep = {
      moved: selected,
      destination,
    };

    const stepResult = stateAnalyzer.tryPerform(step);
    this.updateGameStateOrAlertError(stepResult);
  }

  onUndoSubPlyClicked(): void {
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);

    const undoResult = stateAnalyzer.tryUndoSubPly();
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
    const stateAnalyzer = getStateAnalyzer(this.state.gameState);
    const redoResult = stateAnalyzer.tryPerform(nextAtomic);

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

  onResetGameClicked(): void {
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
          analysisSuggestionDetailLevels: {},
        },
        mctsState: { isRunning: true, mostRecentSnapshot: option.none() },
        thinkingTimeInMS: option.none(),
        thinkingTimeInputValue: "",
        stopTime: option.none(),
      };

      gameStateSaver.setState(gameState);
      futureSubPlyStackSaver.setState(state.ux.futureSubPlyStack);
      thinkingTimeSaver.setState(state.thinkingTimeInMS);

      this.setState(state);
      this.mctsService.updateGameState(gameState, state.thinkingTimeInMS);
    }
  }

  isBestAtomicLegal({ bestAtomic }: MctsAnalysisSnapshot): boolean {
    const legal = getStateAnalyzer(this.state.gameState)
      .tryPerform(bestAtomic)
      .isOk();
    if (!legal) {
      console.log("outdated best atomic", bestAtomic);
    }
    return legal;
  }

  onMctsServiceSnapshot(analysis: Option<MctsAnalysisSnapshot>): void {
    if (this.state.mctsState.isRunning) {
      this.setState({
        mctsState: {
          isRunning: true,
          mostRecentSnapshot: analysis.filter(this.isBestAtomicLegal),
        },
      });
    }
  }

  onMctsServicePause(analyzer: MctsAnalyzer): void {
    this.setState({
      mctsState: { isRunning: false, analyzer, expandedNodeIndexes: [] },
    });

    this.resetAnalysisSuggestionDetailLevels(analyzer);
  }

  resetAnalysisSuggestionDetailLevels(analyzer: MctsAnalyzer) {
    const rootPointer = analyzer.getRootPointer();
    const bestChildPointer = analyzer.getChildPointersFromBestToWorst(
      rootPointer
    )[0];
    const bestChildAtomic = analyzer
      .getNodeSummary(bestChildPointer)
      .atomic.expect("Impossible: Child node has no atomic.");

    if (
      gameUtil.isAnimalStep(bestChildAtomic) &&
      !this.state.gameState.pendingAnimalStep
    ) {
      this.updateUxState({
        analysisSuggestionDetailLevels: {
          [pointerToIndex(rootPointer)]: SuggestionDetailLevel.BestAction,
          [pointerToIndex(bestChildPointer)]: SuggestionDetailLevel.BestAction,
        },
      });
    } else {
      this.updateUxState({
        analysisSuggestionDetailLevels: {
          [pointerToIndex(rootPointer)]: SuggestionDetailLevel.BestAction,
        },
      });
    }
  }

  onPauseAnalyzerClicked(): void {
    this.mctsService.pause();
  }

  onResumeAnalyzerClicked(): void {
    const { mctsState } = this.state;
    if (!mctsState.isRunning) {
      this.setState({
        mctsState: {
          isRunning: true,
          mostRecentSnapshot: option.some(mctsState.analyzer.getSnapshot()),
        },
      });

      this.mctsService.resume(mctsState.analyzer);
    }
  }

  onDetailLevelChange(
    pointer: NodePointer,
    detailLevel: SuggestionDetailLevel
  ): void {
    this.updateUxState({
      analysisSuggestionDetailLevels: {
        ...this.state.ux.analysisSuggestionDetailLevels,
        [pointerToIndex(pointer)]: detailLevel,
      },
    });
  }

  onTimeLimitEnabledChange(event: React.ChangeEvent<HTMLInputElement>): void {
    if (event.target.checked && this.state.thinkingTimeInMS.isNone()) {
      this.saveAndUpdateThinkingTime(option.some(DEFAULT_THINKING_TIME_IN_MS));
    }

    if (!event.target.checked && this.state.thinkingTimeInMS.isSome()) {
      this.saveAndUpdateThinkingTime(option.none());
    }
  }

  saveAndUpdateThinkingTime(thinkingTimeInMS: Option<number>): void {
    this.setState({
      thinkingTimeInMS,
      thinkingTimeInputValue: thinkingTimeInMS.match({
        none: () => "",
        some: (timeInMS) => "" + timeInMS * 1e-3,
      }),
    });
    thinkingTimeSaver.setState(thinkingTimeInMS);
  }

  onThinkingTimeInputChange(event: React.ChangeEvent<HTMLInputElement>): void {
    const parsed = parseInt(event.target.value, 10);
    const parsedInMilliseconds = parsed * 1e3;
    if (parsedInMilliseconds > 0 && parsed === ~~event.target.value) {
      this.saveAndUpdateThinkingTime(option.some(parsedInMilliseconds));
    } else {
      this.setState({ thinkingTimeInputValue: event.target.value });
    }
  }

  onMctsServiceOnStopTimeChange(stopTime: Option<number>) {
    this.setState({ stopTime });
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
  const thinkingTimeInMS: Option<number> = thinkingTimeSaver
    .getState()
    .unwrapOrElse(() => option.none());

  return {
    gameState,
    ux: {
      selectedCardType: option.none(),
      futureSubPlyStack,
      analysisSuggestionDetailLevels: {},
    },
    mctsState: { isRunning: true, mostRecentSnapshot: option.none() },
    thinkingTimeInMS,
    thinkingTimeInputValue: thinkingTimeInMS.match({
      none: () => "",
      some: (timeInMS) => "" + timeInMS * 1e-3,
    }),
    stopTime: option.none(),
  };
}

function isAlpha(card: Card): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: Card): boolean {
  return card.allegiance === Player.Beta;
}
