import React from "react";
import { option, Result } from "rusty-ts";
import "./App.css";
import { cardEmojis } from "./cardMaps";
import CardComponent from "./components/CardComponent";
import ElementMatrix from "./components/ElementMatrix";
import PlyComponent from "./components/PlyComponent";
import SubPlyComponent from "./components/SubPlyComponent";
import { gameUtil } from "./game";
import stateSaver from "./stateSaver";
import {
  AllegiantCard,
  AnimalStep,
  AppState,
  Card,
  CardLocation,
  CardType,
  Drop,
  GameState,
  IllegalGameStateUpdate,
  Player,
  PlyType,
  Row,
  SnipeStep,
} from "./types";

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

    const initialBoard = gameState.getInitialState().getBoard();
    const currentBoard = gameState.getBoard();
    const plies = gameState.getPlies();

    const { selectedCard, futureSubPlyStack } = ux;

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
                    gameState={gameState}
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
                    gameState={gameState}
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
                    gameState={gameState}
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
                    gameState={gameState}
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
                    gameState={gameState}
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
                    gameState={gameState}
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
                    gameState={gameState}
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
                    gameState={gameState}
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
                {getEmoji({
                  cardType: CardType.Snipe,
                  instance: 1,
                }) + "1"}
                .
              </div>{" "}
              =
              {initialBoard[CardLocation.BetaReserve].map((card) =>
                getEmoji(card)
              )}
              ; {initialBoard[CardLocation.Row6].map((card) => getEmoji(card))};{" "}
              {initialBoard[CardLocation.Row5].map((card) => getEmoji(card))};{" "}
              {initialBoard[CardLocation.Row4].map((card) => getEmoji(card))}
            </li>
            <li>
              <div className="PlyNumber">
                {getEmoji({
                  cardType: CardType.Snipe,
                  instance: 0,
                }) + "2"}
                .
              </div>{" "}
              =
              {initialBoard[CardLocation.AlphaReserve].map((card) =>
                getEmoji(card)
              )}
              ; {initialBoard[CardLocation.Row1].map((card) => getEmoji(card))};{" "}
              {initialBoard[CardLocation.Row2].map((card) => getEmoji(card))};{" "}
              {initialBoard[CardLocation.Row3].map((card) => getEmoji(card))}
            </li>

            {plies.map((ply, zeroBasedPlyNumber) => {
              const plyNumber = zeroBasedPlyNumber + 3;
              return <PlyComponent ply={ply} plyNumber={plyNumber} />;
            })}

            {gameState.getPendingAnimalStep().match({
              none: () => null,
              some: (pendingSubPly) =>
                gameState.isGameOver() ? null : (
                  <SubPlyComponent
                    subPly={pendingSubPly}
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
    const selectedCardType = this.state.ux.selectedCard.map(
      (card) => card.cardType
    );
    const snipes = cards.filter((card) => card.cardType === CardType.Snipe);
    return snipes.map((card) => {
      const isSelected = selectedCardType.equalsSome(card.cardType);
      return (
        <CardComponent
          card={card}
          isSelected={isSelected}
          isCapturable={false}
          onCardClicked={this.onCardClicked}
        />
      );
    });
  }

  renderTraditionalView(): React.ReactElement {
    const { gameState } = this.state;
    const currentBoard = gameState.getBoard();

    return (
      <table>
        <tbody>
          <tr>
            <td>Reserve</td>
            <td
              className={
                gameState.isGameOver()
                  ? ""
                  : gameState.getTurn() === Player.Alpha
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
                gameState.isGameOver()
                  ? ""
                  : gameState.getTurn() === Player.Beta
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
        isSelected={this.state.ux.selectedCard.match({
          none: () => false,
          some: (selectedCard) => selectedCard.cardType === card.cardType,
        })}
        isCapturable={false}
        onCardClicked={() => this.onCardClicked(card)}
      />
    ));
  }

  onCardClicked(clicked: Card): void {
    const { selectedCard } = this.state.ux;
    const isClickedCardAlreadySelected = selectedCard.someSatisfies(
      (selected) => gameUtil.areCardsEqual(selected, clicked)
    );
    if (isClickedCardAlreadySelected) {
      this.updateUxState({ selectedCard: option.none() });
    } else {
      this.updateUxState({ selectedCard: option.some(clicked) });
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
    const { gameState } = this.state;
    const { selectedCard } = this.state.ux;

    selectedCard.ifSome((selected) => {
      const location = gameState.getCardLocation(selected);
      if (gameUtil.isReserve(location)) {
        this.tryDrop(selected, row);
      } else {
        this.tryAnimalStep(selected, row);
      }
    });
  }

  tryDrop(selected: Card, destination: Row): void {
    const { gameState } = this.state;
    const drop: Drop = {
      plyType: PlyType.Drop,
      dropped: selected,
      destination,
    };

    const dropResult = gameState.tryDrop(drop);
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

  tryAnimalStep(selected: Card, destination: Row): void {
    const { gameState } = this.state;
    const step: AnimalStep = {
      moved: selected,
      destination,
    };

    const stepResult = gameState.tryAnimalStep(step);
    this.updateGameStateOrAlertError(stepResult);
  }

  onUndoSubPlyClicked(): void {
    const { gameState } = this.state;

    const undoResult = gameState.tryUndoSubPly();
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

    const { gameState } = this.state;
    const stack = this.state.ux.futureSubPlyStack;

    const redoResult = stack.pendingAnimalStep.match({
      some: (step) => {
        this.updateUxState({
          futureSubPlyStack: {
            pendingAnimalStep: option.none(),
            plies: this.state.ux.futureSubPlyStack.plies,
          },
        });

        return gameState.tryPerform(step);
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

          return gameState.tryPerform(nextPly.first);
        } else {
          this.updateUxState({
            futureSubPlyStack: {
              pendingAnimalStep: option.none(),
              plies: newPlies,
            },
          });

          return gameState.tryPerform(nextPly);
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
          selectedCard: option.none(),
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
      selectedCard: option.none(),
      futureSubPlyStack: { pendingAnimalStep: option.none(), plies: [] },
    },
  };
}

function getEmoji(card: Card): string {
  if (card.cardType === CardType.Snipe) {
    switch (card.instance) {
      case 0:
        return "α";
      case 1:
        return "β";
    }
  } else {
    return cardEmojis[card.cardType];
  }
}

function isAlpha(card: AllegiantCard): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: AllegiantCard): boolean {
  return card.allegiance === Player.Beta;
}
