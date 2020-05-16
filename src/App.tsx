import React from "react";
import { option } from "rusty-ts";
import "./App.css";
import CardComponent from "./components/CardComponent";
import { getRandomState, tryCapture } from "./game";
import stateSaver from "./stateSaver";
import { AppState, Card, Player, CardType } from "./types";

export default class App extends React.Component<{}, AppState> {
  constructor(props: {}) {
    super(props);

    this.state = loadState();
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
                <td>1</td>
                <td>
                  {this.renderCards(gameState.alpha.backRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.alpha.backRow.filter(isAlpha))}
                </td>
              </tr>
              <tr>
                <td>2</td>
                <td>
                  {this.renderCards(gameState.alpha.frontRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.alpha.frontRow.filter(isAlpha))}
                </td>
              </tr>
              <tr>
                <td>3</td>
                <td>
                  {this.renderCards(gameState.beta.frontRow.filter(isBeta))}
                </td>
                <td>
                  {this.renderCards(gameState.beta.frontRow.filter(isAlpha))}
                </td>
              </tr>{" "}
              <tr>
                <td>4</td>
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
        this.saveState({ gameState: newGameState });
      },
      err: (e) => {
        alert(e);
      },
    });
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
