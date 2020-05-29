import React from "react";
import { option, Result } from "rusty-ts";
import "./App.css";
import { cardEmojis } from "./cardMaps";
import CardComponent from "./components/CardComponent";
import ElementMatrix from "./components/ElementMatrix";
import PlyComponent from "./components/PlyComponent";
import AnimalStepComponent from "./components/AnimalStepComponent";
import * as gameUtil from "./gameUtil";
import stateSaver from "./stateSaver";
import {
  Card,
  AnimalStep,
  AppState,
  CardType,
  CardLocation,
  Drop,
  GameAnalyzer,
  IllegalGameStateUpdate,
  Player,
  PlyType,
  Row,
  SnipeStep,
  GameState,
  AnimalType,
} from "./types";
import { getAnalyzer } from "./analyzer";

export default class App extends React.Component<{}, AppState> {
  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();
  }

  bindMethods() {
    this.onCardClicked = this.onCardClicked.bind(this);
    this.onResetClicked = this.onResetClicked.bind(this);
    this.onUndoSubPlyClicked = this.onUndoSubPlyClicked.bind(this);
    this.onRedoSubPlyClicked = this.onRedoSubPlyClicked.bind(this);
  }

  saveAndUpdateGameState(newGameState: GameState) {
    stateSaver.setState(newGameState);
    this.setState({ gameState: newGameState });
  }

  render(): React.ReactElement {
    return this.renderMatrixView();
  }

  renderMatrixView(): React.ReactElement {
    const { gameState, ux } = this.state;

    const analyzer = getAnalyzer(this.state.gameState);
    const initialBoard = getAnalyzer(analyzer.getInitialState()).getBoard();
    const currentBoard = analyzer.getBoard();
    const plies = analyzer.getPlies();

    const { selectedCardType: selectedCard, futureSubPlyStack } = ux;

    return (
      <div className="SnipeHunt">
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
                  onClick={() => this.onRowNumberClicked(1)}
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
                  onClick={() => this.onRowNumberClicked(2)}
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
                  onClick={() => this.onRowNumberClicked(3)}
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
                  onClick={() => this.onRowNumberClicked(4)}
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
                  onClick={() => this.onRowNumberClicked(5)}
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
                  onClick={() => this.onRowNumberClicked(6)}
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
              return <PlyComponent ply={ply} plyNumber={plyNumber} />;
            })}

            {analyzer.getPendingAnimalStep().match({
              none: () => null,
              some: (step) =>
                analyzer.isGameOver() ? null : (
                  <AnimalStepComponent
                    step={step}
                    plyNumber={plies.length + 3}
                  />
                ),
            })}
          </ol>
          <h4>Future sub plies</h4>
          <ol className="Plies">
            {futureSubPlyStack.pendingAnimalStep.match({
              some: () => <p>Todo: future animal step</p>,
              none: () => null,
            })}
            {futureSubPlyStack.plies
              .slice()
              .reverse()
              .map((ply, i) => (
                <PlyComponent ply={ply} plyNumber={plies.length + 3 + i} />
              ))}
          </ol>
          <button onClick={this.onUndoSubPlyClicked}>Back</button>
          <button onClick={this.onRedoSubPlyClicked}>Forward</button>
        </div>
        <button onClick={this.onResetClicked}>Reset</button>
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
        <CardComponent
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
      <CardComponent
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

  onCardClicked(clicked: Card): void {
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
      return {
        ...prevState,
        ux: {
          ...prevState.ux,
          ...newUxState,
        },
      };
    });
  }

  onRowNumberClicked(row: Row): void {
    const analyzer = getAnalyzer(this.state.gameState);

    if (analyzer.isGameOver()) {
      return;
    }

    const { selectedCardType: selectedCard } = this.state.ux;

    selectedCard.ifSome((selected) => {
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
      },
      err: (errorCode) => {
        alert(IllegalGameStateUpdate[errorCode]);
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
          futureSubPlyStack: this.getNewFutureSubPlyStack(undone),
        });

        this.saveAndUpdateGameState(newGameState);
      },

      err: (errorCode) => {
        alert(IllegalGameStateUpdate[errorCode]);
      },
    });
  }

  getNewFutureSubPlyStack(
    undone: SnipeStep | Drop | AnimalStep
  ): AppState["ux"]["futureSubPlyStack"] {
    const stack = this.state.ux.futureSubPlyStack;

    return stack.pendingAnimalStep.match({
      some: (pending) => {
        if ("plyType" in undone) {
          throw new Error(
            "Unreachable: Cannot undo a non-animal-step atomic if there is an animal step was just undone."
          );
        }
        return {
          pendingAnimalStep: option.none(),
          plies: stack.plies.concat([
            { plyType: PlyType.TwoAnimalSteps, first: undone, second: pending },
          ]),
        };
      },

      none: () => {
        if ("plyType" in undone) {
          return {
            pendingAnimalStep: option.none(),
            plies: stack.plies.concat([undone]),
          };
        }

        return {
          pendingAnimalStep: option.none(),
          plies: stack.plies,
        };
      },
    });
  }

  onRedoSubPlyClicked(): void {
    if (!this.canRedoSubPly()) {
      alert("Nothing to redo.");
    }

    const analyzer = getAnalyzer(this.state.gameState);
    const stack = this.state.ux.futureSubPlyStack;

    const redoResult = stack.pendingAnimalStep.match({
      some: (step) => {
        this.updateUxState({
          futureSubPlyStack: {
            pendingAnimalStep: option.none(),
            plies: this.state.ux.futureSubPlyStack.plies,
          },
        });

        return analyzer.tryPerform(step);
      },

      none: () => {
        const nextPly = stack.plies[stack.plies.length - 1];
        const newPlies = stack.plies.slice(0, -1);

        if (nextPly.plyType === PlyType.TwoAnimalSteps) {
          this.updateUxState({
            futureSubPlyStack: {
              pendingAnimalStep: option.some(nextPly.second),
              plies: newPlies,
            },
          });

          return analyzer.tryPerform(nextPly.first);
        } else {
          this.updateUxState({
            futureSubPlyStack: {
              pendingAnimalStep: option.none(),
              plies: newPlies,
            },
          });

          return analyzer.tryPerform(nextPly);
        }
      },
    });

    this.updateGameStateOrAlertError(redoResult);
  }

  canRedoSubPly(): boolean {
    const stack = this.state.ux.futureSubPlyStack;
    return stack.pendingAnimalStep.isSome() || stack.plies.length > 0;
  }

  onResetClicked(): void {
    if (window.confirm("Are you sure you want to reset?")) {
      const state: AppState = {
        gameState: gameUtil.getRandomGameState(),
        ux: {
          selectedCardType: option.none(),
          futureSubPlyStack: { pendingAnimalStep: option.none(), plies: [] },
        },
      };
      stateSaver.setState(state.gameState);
      this.setState(state);
    }
  }
}

function loadState(): AppState {
  const gameState = stateSaver.getState().unwrapOrElse(() => {
    const newGameState = gameUtil.getRandomGameState();
    stateSaver.setState(newGameState);
    return newGameState;
  });

  return {
    gameState: gameState,
    ux: {
      selectedCardType: option.none(),
      futureSubPlyStack: { pendingAnimalStep: option.none(), plies: [] },
    },
  };
}

function isAlpha(card: Card): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: Card): boolean {
  return card.allegiance === Player.Beta;
}
