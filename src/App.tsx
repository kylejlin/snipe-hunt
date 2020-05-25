import React from "react";
import { option } from "rusty-ts";
import "./App.css";
import { cardEmojis } from "./cardMaps";
import CardComponent from "./components/CardComponent";
import ElementMatrix from "./components/ElementMatrix";
import PlyComponent from "./components/PlyComponent";
import SubPlyComponent from "./components/SubPlyComponent";
import stateSaver from "./stateSaver";
import { AppState, Card, CardType, Player, CardLocation, Row } from "./types";
import { gameUtil } from "./game";

export default class App<X_T> extends React.Component<{}, AppState> {
  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();
  }

  bindMethods() {
    this.onCardClicked = this.onCardClicked.bind(this);
    this.onResetClicked = this.onResetClicked.bind(this);
    this.onUndoPlyClicked = this.onUndoPlyClicked.bind(this);
    this.onRedoPlyClicked = this.onRedoPlyClicked.bind(this);
  }

  saveState(
    stateUpdatesOrUpdater:
      | Partial<AppState>
      | ((prevState: AppState) => AppState)
  ): void {
    if ("function" === typeof stateUpdatesOrUpdater) {
      stateSaver.setState(stateUpdatesOrUpdater(this.state).gameState);
      this.setState(stateUpdatesOrUpdater);
    } else {
      const newState = { ...this.state, ...stateUpdatesOrUpdater };
      stateSaver.setState(newState.gameState);
      this.setState(stateUpdatesOrUpdater as AppState);
    }
  }

  render(): React.ReactElement {
    return this.renderMatrixView();
  }

  renderMatrixView(): React.ReactElement {
    const { gameState, ux } = this.state;

    const initialBoard = gameState.getInitialState().getBoard();
    const currentBoard = gameState.getBoard();
    const plies = gameState.getPlies();

    const { selectedCard, futurePlyStack } = ux;

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
                  allegiance: Player.Beta,
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
                  allegiance: Player.Alpha,
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
          <h4>Future plies</h4>
          <ol className="Plies">
            {futurePlyStack
              .slice()
              .reverse()
              .map((ply, i) => (
                <PlyComponent ply={ply} plyNumber={plies.length + 3 + i} />
              ))}
          </ol>
          <button onClick={this.onUndoPlyClicked}>Back</button>
          <button onClick={this.onRedoPlyClicked}>Forward</button>
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
    this.state.ux.selectedCard.match({
      none: () => {
        this.saveState((prevState) => {
          return {
            ...prevState,
            ux: { ...prevState.ux, selectedCard: option.some(clicked) },
          };
        });
      },
      some: (selected) => {
        if (gameUtil.areCardsEqual(selected, clicked)) {
          this.saveState((prevState) => {
            return {
              ...prevState,
              ux: { ...prevState.ux, selectedCard: option.none() },
            };
          });
        } else {
          this.tryCapture(selected.cardType, clicked.cardType);
        }
      },
    });
  }

  tryCapture(attacker: CardType, target: CardType): void {
    // tryCapture(this.state.gameState, attacker, target).match({
    //   ok: (newGameState) => {
    //     this.saveState({
    //       gameState: newGameState,
    //       selectedCard: option.none(),
    //     });
    //   },
    //   err: (e) => {
    //     alert(IllegalMove[e]);
    //     this.saveState({ selectedCard: option.none() });
    //   },
    // });
  }

  onRowNumberClicked(row: Row): void {
    this.state.ux.selectedCard.ifSome((selected) => {
      // getRowNumber(this.state.gameState, selected).match({
      //   none: () => {
      //     this.tryDrop(selected, row);
      //   },
      //   some: (selectedCardRow) => {
      //     if (selectedCardRow === row) {
      //       this.tryToggle(selected);
      //     } else {
      //       this.tryMove(selected, row);
      //     }
      //   },
      // });
    });
  }

  tryMove(selectedCard: CardType, row: Row): void {
    // tryMove(this.state.gameState, selectedCard, row).match({
    //   ok: (newGameState) => {
    //     this.saveState({
    //       gameState: newGameState,
    //       selectedCard: option.none(),
    //     });
    //   },
    //   err: (e) => {
    //     alert(IllegalMove[e]);
    //     this.saveState({ selectedCard: option.none() });
    //   },
    // });
  }

  tryDrop(selectedCard: CardType, destination: Row): void {
    // tryDrop(this.state.gameState, selectedCard, destination).match({
    //   ok: (newGameState) => {
    //     this.saveState({
    //       gameState: newGameState,
    //       selectedCard: option.none(),
    //     });
    //   },
    //   err: (e) => {
    //     alert(IllegalDrop[e]);
    //     this.saveState({ selectedCard: option.none() });
    //   },
    // });
  }

  tryToggle(selectedCard: CardType): void {
    // tryToggle(this.state.gameState, selectedCard).match({
    //   ok: (newGameState) => {
    //     this.saveState({
    //       gameState: newGameState,
    //       selectedCard: option.none(),
    //     });
    //   },
    //   err: (e) => {
    //     alert(IllegalToggle[e]);
    //     this.saveState({ selectedCard: option.none() });
    //   },
    // });
  }

  onUndoPlyClicked(): void {
    // tryUndoPlyOrSubPly(this.state.gameState).match({
    //   ok: (newGameState) => {
    //     this.saveState({
    //       gameState: newGameState,
    //     });
    //   },
    //   err: (e) => {
    //     alert(IllegalUndo[e]);
    //     this.saveState({ selectedCard: option.none() });
    //   },
    // });
  }

  onRedoPlyClicked(): void {
    // const { gameState } = this.state;
    // if (gameState.futurePlyStack.length > 0) {
    //   recalculateOutOfSyncGameState({
    //     ...gameState,
    //     plies: gameState.plies.concat(gameState.futurePlyStack.slice(-1)),
    //     futurePlyStack: gameState.futurePlyStack.slice(0, -1),
    //   }).match({
    //     ok: (newGameState) => {
    //       this.saveState({
    //         gameState: newGameState,
    //       });
    //     },
    //     err: (e) => {
    //       alert("Bug: Illegal redo");
    //       this.saveState({ selectedCard: option.none() });
    //     },
    //   });
    // }
  }

  onResetClicked(): void {
    if (window.confirm("Are you sure you want to reset?")) {
      const state: AppState = {
        gameState: gameUtil.getRandomGameState(),
        ux: { selectedCard: option.none(), futurePlyStack: [] },
      };
      this.saveState(state);
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
    ux: { selectedCard: option.none(), futurePlyStack: [] },
  };
}

function getEmoji(card: Card): string {
  if (card.cardType === CardType.Snipe) {
    switch (card.allegiance) {
      case Player.Alpha:
        return "α";
      case Player.Beta:
        return "β";
    }
  } else {
    return cardEmojis[card.cardType];
  }
}

function isAlpha(card: Card): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: Card): boolean {
  return card.allegiance === Player.Beta;
}
