import React from "react";
import { option } from "rusty-ts";
import "./App.css";
import { cardEmojis } from "./cardMaps";
import CardComponent from "./components/CardComponent";
import ElementMatrix from "./components/ElementMatrix";
import PlyComponent from "./components/PlyComponent";
import SubPlyComponent from "./components/SubPlyComponent";
import {
  getRandomState,
  getRowNumber,
  IllegalDrop,
  IllegalMove,
  IllegalToggle,
  IllegalUndo,
  isGameOver,
  isSnipe,
  tryCapture,
  tryDrop,
  tryMove,
  tryToggle,
  tryUndoPlyOrSubPly,
  recalculateOutOfSyncGameState,
} from "./OLD_game";
import stateSaver from "./stateSaver";
import { AppState, Card, CardType, Player, RowNumber } from "./types";
import { getBestPly, getLegalPlies } from "./kraken";
import { getGameAnalyzer } from "./game";

export default class App extends React.Component<{}, AppState> {
  constructor(props: {}) {
    super(props);

    this.state = loadState();

    this.bindMethods();

    (window as any).app = this;
    (window as any).getBestPly = getBestPly;
    (window as any).getLegalPlies = getLegalPlies;
  }

  bindMethods() {
    this.onCardClicked = this.onCardClicked.bind(this);
    this.onResetClicked = this.onResetClicked.bind(this);
    this.onUndoPlyClicked = this.onUndoPlyClicked.bind(this);
    this.onRedoPlyClicked = this.onRedoPlyClicked.bind(this);
  }

  saveState(newState: Partial<AppState>): void {
    this.setState(newState as AppState);
    stateSaver.setState({ ...this.state, ...newState });
  }

  render(): React.ReactElement {
    return this.renderMatrixView();
  }

  renderMatrixView(): React.ReactElement {
    const { gameState, ux } = this.state;

    const analyzer = getGameAnalyzer(gameState);
    const cards = analyzer.getBoardCards();

    const { selectedCardType } = ux;

    return (
      <div className="SnipeHunt">
        <div>
          <table className="Board">
            <tbody>
              <tr>
                <td className="BoardCell">
                  Reserve{this.renderSnipesIn(cards.alphaReserve)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.alphaReserve}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(1)}
                >
                  1{this.renderSnipesIn(cards.rows[1])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.rows[1]}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(2)}
                >
                  2{this.renderSnipesIn(cards.rows[2])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.rows[2]}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(3)}
                >
                  3{this.renderSnipesIn(cards.rows[3])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.rows[3]}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(4)}
                >
                  4{this.renderSnipesIn(cards.rows[4])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.rows[4]}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(5)}
                >
                  5{this.renderSnipesIn(cards.rows[5])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.rows[5]}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td
                  className="BoardCell"
                  onClick={() => this.onRowNumberClicked(6)}
                >
                  6{this.renderSnipesIn(cards.rows[6])}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.rows[6]}
                    selectedCardType={selectedCardType}
                    onCardClicked={this.onCardClicked}
                  />
                </td>
              </tr>
              <tr>
                <td className="BoardCell">
                  Reserve{this.renderSnipesIn(cards.betaReserve)}
                </td>
                <td className="BoardCell">
                  <ElementMatrix
                    gameState={gameState}
                    cards={cards.betaReserve}
                    selectedCardType={selectedCardType}
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
                  instance: 0,
                  allegiance: Player.Beta,
                }) + "1"}
                .
              </div>{" "}
              =
              {gameState.initialPositions.beta.reserve.map((card) =>
                getEmoji(card)
              )}
              ;{" "}
              {gameState.initialPositions.beta.backRow.map((card) =>
                getEmoji(card)
              )}
              ;{" "}
              {gameState.initialPositions.beta.frontRow.map((card) =>
                getEmoji(card)
              )}
            </li>
            <li>
              <div className="PlyNumber">
                {getEmoji({
                  cardType: CardType.Snipe,
                  instance: 0,
                  allegiance: Player.Alpha,
                }) + "2"}
                .
              </div>{" "}
              =
              {gameState.initialPositions.alpha.reserve.map((card) =>
                getEmoji(card)
              )}
              ;{" "}
              {gameState.initialPositions.alpha.backRow.map((card) =>
                getEmoji(card)
              )}
              ;{" "}
              {gameState.initialPositions.alpha.frontRow.map((card) =>
                getEmoji(card)
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
          <h4>Future plies</h4>
          <ol className="Plies">
            {gameState.futurePlyStack
              .slice()
              .reverse()
              .map((ply, i) => (
                <PlyComponent
                  ply={ply}
                  plyNumber={gameState.plies.length + 3 + i}
                />
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
    const snipes = cards.filter((card) => isSnipe(card.cardType));
    return snipes.map((card) => {
      const isSelected =
        this.state.selectedCard.unwrapOr(null) === card.cardType;
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
        isCapturable={false}
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

  onRowNumberClicked(row: RowNumber): void {
    this.state.selectedCard.ifSome((selectedCard) => {
      getRowNumber(this.state.gameState, selectedCard).match({
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

  tryMove(selectedCard: CardType, row: RowNumber): void {
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

  tryDrop(selectedCard: CardType, destination: RowNumber): void {
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

  onUndoPlyClicked(): void {
    tryUndoPlyOrSubPly(this.state.gameState).match({
      ok: (newGameState) => {
        this.saveState({
          gameState: newGameState,
        });
      },
      err: (e) => {
        alert(IllegalUndo[e]);
        this.saveState({ selectedCard: option.none() });
      },
    });
  }

  onRedoPlyClicked(): void {
    const { gameState } = this.state;
    if (gameState.futurePlyStack.length > 0) {
      recalculateOutOfSyncGameState({
        ...gameState,
        plies: gameState.plies.concat(gameState.futurePlyStack.slice(-1)),
        futurePlyStack: gameState.futurePlyStack.slice(0, -1),
      }).match({
        ok: (newGameState) => {
          this.saveState({
            gameState: newGameState,
          });
        },
        err: (e) => {
          alert("Bug: Illegal redo");
          this.saveState({ selectedCard: option.none() });
        },
      });
    }
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
