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
} from "./game";
import stateSaver from "./stateSaver";
import { AppState, Card, CardType, Player, Row } from "./types";

export default class App extends React.Component<{}, AppState> {
  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();
  }

  bindMethods() {
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
        <div className="Board">
          <table>
            <tbody>
              <tr>
                <td>Reserve</td>
                <td>
                  <ElementMatrix
                    cards={gameState.alpha.reserve}
                    showInactiveElements={false}
                  />
                </td>
                <td>
                  <ElementMatrix
                    cards={gameState.alpha.reserve}
                    showInactiveElements={true}
                  />
                </td>
              </tr>
              <tr>
                <td>1</td>
                <td>
                  <ElementMatrix
                    cards={gameState.alpha.backRow}
                    showInactiveElements={false}
                  />
                </td>
                <td>
                  <ElementMatrix
                    cards={gameState.alpha.backRow}
                    showInactiveElements={true}
                  />
                </td>
              </tr>
              <tr>
                <td>2</td>
                <td>
                  <ElementMatrix
                    cards={gameState.alpha.frontRow}
                    showInactiveElements={false}
                  />
                </td>
                <td>
                  <ElementMatrix
                    cards={gameState.alpha.frontRow}
                    showInactiveElements={true}
                  />
                </td>
              </tr>
              <tr>
                <td>3</td>
                <td>
                  <ElementMatrix
                    cards={gameState.beta.frontRow}
                    showInactiveElements={false}
                  />
                </td>
                <td>
                  <ElementMatrix
                    cards={gameState.beta.frontRow}
                    showInactiveElements={true}
                  />
                </td>
              </tr>
              <tr>
                <td>4</td>
                <td>
                  <ElementMatrix
                    cards={gameState.beta.backRow}
                    showInactiveElements={false}
                  />
                </td>
                <td>
                  <ElementMatrix
                    cards={gameState.beta.backRow}
                    showInactiveElements={true}
                  />
                </td>
              </tr>
              <tr>
                <td>Reserve</td>
                <td>
                  <ElementMatrix
                    cards={gameState.beta.reserve}
                    showInactiveElements={false}
                  />
                </td>
                <td>
                  <ElementMatrix
                    cards={gameState.beta.reserve}
                    showInactiveElements={true}
                  />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div className="Moves"></div>
        <button onClick={this.onResetClicked}>Reset</button>
      </div>
    );
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
        onSelect={() => this.onSelectCard(card)}
      />
    ));
  }

  onSelectCard(card: Card): void {
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
        alert(e);
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
        alert(e);
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
        alert(e);
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
        alert(e);
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

function isAlpha(card: Card): boolean {
  return card.allegiance === Player.Alpha;
}

function isBeta(card: Card): boolean {
  return card.allegiance === Player.Beta;
}
