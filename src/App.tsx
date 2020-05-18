import React from "react";
import { option } from "rusty-ts";
import "./App.css";
import CardComponent from "./components/CardComponent";
import ElementMatrix from "./components/ElementMatrix";
import {
  getRandomState,
  getRow,
  tryCapture,
  tryDrop,
  tryMove,
  tryToggle,
  isGameOver,
  IllegalMove,
  IllegalDrop,
  IllegalToggle,
  isSnipe,
} from "./game";
import stateSaver from "./stateSaver";
import { AppState, Card, CardType, Player, Row, PlyType } from "./types";
import { cardEmojis } from "./cardMaps";
import PlyComponent from "./components/PlyComponent";
import SubPlyComponent from "./components/SubPlyComponent";

export default class App extends React.Component<{}, AppState> {
  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();
  }

  bindMethods() {
    this.onCardClicked = this.onCardClicked.bind(this);
    this.onResetClicked = this.onResetClicked.bind(this);
  }

  saveState(newState: Partial<AppState>): void {
    this.setState(newState as AppState);
    stateSaver.setState({ ...this.state, ...newState });
  }

  render(): React.ReactElement {
    return this.renderMatrixView();
  }

  renderMatrixView(): React.ReactElement {
    const { gameState } = this.state;
    return (
      <div className="SnipeHunt">
        <div>
          <table className="Board">
            <tbody>
              <tr>
                <td className="BoardCell">
                  Reserve{this.renderSnipesIn(gameState.alpha.reserve)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.alpha.reserve}
                    showInactiveElements={false}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.alpha.reserve}
                    showInactiveElements={true}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(1)}
                >
                  1{this.renderSnipesIn(gameState.alpha.backRow)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.alpha.backRow}
                    showInactiveElements={false}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.alpha.backRow}
                    showInactiveElements={true}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(2)}
                >
                  2{this.renderSnipesIn(gameState.alpha.frontRow)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.alpha.frontRow}
                    showInactiveElements={false}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.alpha.frontRow}
                    showInactiveElements={true}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(3)}
                >
                  3{this.renderSnipesIn(gameState.beta.frontRow)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.beta.frontRow}
                    showInactiveElements={false}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.beta.frontRow}
                    showInactiveElements={true}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(4)}
                >
                  4{this.renderSnipesIn(gameState.beta.backRow)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.beta.backRow}
                    showInactiveElements={false}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.beta.backRow}
                    showInactiveElements={true}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td className="BoardCell">
                  Reserve{this.renderSnipesIn(gameState.beta.reserve)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.beta.reserve}
                    showInactiveElements={false}
                    selectedCard={this.state.selectedCard}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    cards={gameState.beta.reserve}
                    showInactiveElements={true}
                    selectedCard={this.state.selectedCard}
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
                {getEmoji(CardType.BetaSnipe) + "1"}.
              </div>{" "}
              =
              {gameState.initialPositions.beta.reserve.map((card) =>
                getEmoji(card.cardType)
              )}
              ;{" "}
              {gameState.initialPositions.beta.backRow.map((card) =>
                getEmoji(card.cardType)
              )}
              ;{" "}
              {gameState.initialPositions.beta.frontRow.map((card) =>
                getEmoji(card.cardType)
              )}
            </li>
            <li>
              <div className="PlyNumber">
                {getEmoji(CardType.AlphaSnipe) + "2"}.
              </div>{" "}
              =
              {gameState.initialPositions.alpha.reserve.map((card) =>
                getEmoji(card.cardType)
              )}
              ;{" "}
              {gameState.initialPositions.alpha.backRow.map((card) =>
                getEmoji(card.cardType)
              )}
              ;{" "}
              {gameState.initialPositions.alpha.frontRow.map((card) =>
                getEmoji(card.cardType)
              )}
            </li>

            {gameState.plies.map((ply, zeroBasedPlyNumber) => {
              const plyNumber = zeroBasedPlyNumber + 3;
              return <PlyComponent ply={ply} plyNumber={plyNumber} />;
            })}

            {gameState.pendingSubPly.match({
              none: () => null,
              some: (pendingSubPly) =>
                isGameOver(gameState) ? null : (
                  <SubPlyComponent
                    subPly={pendingSubPly}
                    plyNumber={gameState.plies.length + 3}
                  />
                ),
            })}
          </ol>
        </div>
        <button onClick={this.onResetClicked}>Reset</button>
      </div>
    );
  }

  renderSnipesIn(cards: Card[]): React.ReactElement[] {
    const snipes = cards.filter((card) => isSnipe(card.cardType));
    return snipes.map((card) => {
      const isSelected =
        this.state.selectedCard.unwrapOr(null) === card.cardType;
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
    const { gameState } = this.state;
    return (
      <table>
        <tbody>
          <tr>
            <td>Reserve</td>
            <td
              className={
                isGameOver(gameState)
                  ? ""
                  : gameState.turn === Player.Alpha
                  ? "TurnIndicatorLight"
                  : ""
              }
            />
            <td>{this.renderCards(gameState.alpha.reserve)}</td>
          </tr>
          <tr>
            <td onClick={() => this.onRowNumberClicked(1)}>1</td>
            <td>{this.renderCards(gameState.alpha.backRow.filter(isBeta))}</td>
            <td>{this.renderCards(gameState.alpha.backRow.filter(isAlpha))}</td>
          </tr>
          <tr>
            <td onClick={() => this.onRowNumberClicked(2)}>2</td>
            <td>{this.renderCards(gameState.alpha.frontRow.filter(isBeta))}</td>
            <td>
              {this.renderCards(gameState.alpha.frontRow.filter(isAlpha))}
            </td>
          </tr>
          <tr>
            <td onClick={() => this.onRowNumberClicked(3)}>3</td>
            <td>{this.renderCards(gameState.beta.frontRow.filter(isBeta))}</td>
            <td>{this.renderCards(gameState.beta.frontRow.filter(isAlpha))}</td>
          </tr>{" "}
          <tr>
            <td onClick={() => this.onRowNumberClicked(4)}>4</td>
            <td>{this.renderCards(gameState.beta.backRow.filter(isBeta))}</td>
            <td>{this.renderCards(gameState.beta.backRow.filter(isAlpha))}</td>
          </tr>
          <tr>
            <td>Reserve</td>
            <td
              className={
                isGameOver(gameState)
                  ? ""
                  : gameState.turn === Player.Beta
                  ? "TurnIndicatorLight"
                  : ""
              }
            />
            <td>{this.renderCards(gameState.beta.reserve)}</td>
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
        isSelected={this.state.selectedCard.match({
          none: () => false,
          some: (cardType) => cardType === card.cardType,
        })}
        onCardClicked={() => this.onCardClicked(card)}
      />
    ));
  }

  onCardClicked(card: Card): void {
    this.state.selectedCard.match({
      none: () => {
        this.saveState({ selectedCard: option.some(card.cardType) });
      },
      some: (currentSelectedCardType) => {
        if (currentSelectedCardType === card.cardType) {
          this.saveState({ selectedCard: option.none() });
        } else {
          this.tryCapture(currentSelectedCardType, card.cardType);
        }
      },
    });
  }

  tryCapture(attacker: CardType, target: CardType): void {
    tryCapture(this.state.gameState, attacker, target).match({
      ok: (newGameState) => {
        this.saveState({
          gameState: newGameState,
          selectedCard: option.none(),
        });
      },
      err: (e) => {
        alert(IllegalMove[e]);
        this.saveState({ selectedCard: option.none() });
      },
    });
  }

  onRowNumberClicked(row: Row): void {
    this.state.selectedCard.ifSome((selectedCard) => {
      getRow(this.state.gameState, selectedCard).match({
        none: () => {
          this.tryDrop(selectedCard, row);
        },
        some: (selectedCardRow) => {
          if (selectedCardRow === row) {
            this.tryToggle(selectedCard);
          } else {
            this.tryMove(selectedCard, row);
          }
        },
      });
    });
  }

  tryMove(selectedCard: CardType, row: Row): void {
    tryMove(this.state.gameState, selectedCard, row).match({
      ok: (newGameState) => {
        this.saveState({
          gameState: newGameState,
          selectedCard: option.none(),
        });
      },
      err: (e) => {
        alert(IllegalMove[e]);
        this.saveState({ selectedCard: option.none() });
      },
    });
  }

  tryDrop(selectedCard: CardType, destination: Row): void {
    tryDrop(this.state.gameState, selectedCard, destination).match({
      ok: (newGameState) => {
        this.saveState({
          gameState: newGameState,
          selectedCard: option.none(),
        });
      },
      err: (e) => {
        alert(IllegalDrop[e]);
        this.saveState({ selectedCard: option.none() });
      },
    });
  }

  tryToggle(selectedCard: CardType): void {
    tryToggle(this.state.gameState, selectedCard).match({
      ok: (newGameState) => {
        this.saveState({
          gameState: newGameState,
          selectedCard: option.none(),
        });
      },
      err: (e) => {
        alert(IllegalToggle[e]);
        this.saveState({ selectedCard: option.none() });
      },
    });
  }

  onResetClicked(): void {
    if (window.confirm("Are you sure you want to reset?")) {
      const state = getRandomState();
      this.saveState(state);
    }
  }
}

function loadState(): AppState {
  return stateSaver.getState().unwrapOrElse(() => {
    const state = getRandomState();
    stateSaver.setState(state);
    return state;
  });
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}

function isAlpha(card: Card): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: Card): boolean {
  return card.allegiance === Player.Beta;
}
