import React from "react";
import { option } from "rusty-ts";
import "./App.css";
import CardComponent from "./components/CardComponent";
import { getRandomState, tryCapture, tryMove } from "./game";
import stateSaver from "./stateSaver";
import { AppState, Card, Player, CardType, Row } from "./types";

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
    const { gameState } = this.state;
    return (
      <div className="SnipeHunt">
        <div className="Board">
          <table>
            <tbody>
              <tr>
                <td>{this.renderCards(gameState.alpha.reserve)}</td>
              </tr>
              <tr>
                <td onClick={() => this.onRowNumberClicked(1)}>1</td>
                <td>
                  {this.renderCards(gameState.alpha.backRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.alpha.backRow.filter(isAlpha))}
                </td>
              </tr>
              <tr>
                <td onClick={() => this.onRowNumberClicked(2)}>2</td>
                <td>
                  {this.renderCards(gameState.alpha.frontRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.alpha.frontRow.filter(isAlpha))}
                </td>
              </tr>
              <tr>
                <td onClick={() => this.onRowNumberClicked(3)}>3</td>
                <td>
                  {this.renderCards(gameState.beta.frontRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.beta.frontRow.filter(isAlpha))}
                </td>
              </tr>{" "}
              <tr>
                <td onClick={() => this.onRowNumberClicked(4)}>4</td>
                <td>
                  {this.renderCards(gameState.beta.backRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.beta.backRow.filter(isAlpha))}
                </td>
              </tr>
              <tr>
                <td>{this.renderCards(gameState.beta.reserve)}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div className="Moves"></div>
        <button onClick={this.onResetClicked}>Reset</button>
      </div>
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
      },
    });
  }

  onRowNumberClicked(row: Row): void {
    this.state.selectedCard.ifSome((selectedCard) => {
      this.tryMove(selectedCard, row);
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
