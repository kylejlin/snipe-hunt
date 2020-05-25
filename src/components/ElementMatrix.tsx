import React from "react";
import { Option } from "rusty-ts";
import {
  canCapture,
  getCardsWithActiveElements,
  getCardsWithInactiveElements,
} from "../OLD_game";
import { Card, CardType, Element, GameState } from "../types";
import CardComponent from "./CardComponent";
import "./styles/ElementMatrix.css";

interface Props {
  gameState: GameState;
  cards: Card[];
  selectedCard: Option<CardType>;
  onCardClicked(card: Card): void;
}

export default function ElementMatrix({
  gameState,
  cards,
  showInactiveElements,
  selectedCard,
  onCardClicked,
}: Props): React.ReactElement {
  interface ElementMatrixCellProps {
    amount: 1 | 2 | 3;
    element: Element;
  }

  return (
    <table className="ElementMatrix">
      <tbody>
        <tr>
          <ElementMatrixCell amount={1} element={Element.Fire} />
          <ElementMatrixCell amount={1} element={Element.Water} />
          <ElementMatrixCell amount={1} element={Element.Earth} />
          <ElementMatrixCell amount={1} element={Element.Air} />
        </tr>
        <tr>
          <ElementMatrixCell amount={2} element={Element.Fire} />
          <ElementMatrixCell amount={2} element={Element.Water} />
          <ElementMatrixCell amount={2} element={Element.Earth} />
          <ElementMatrixCell amount={2} element={Element.Air} />
        </tr>
        <tr>
          <ElementMatrixCell amount={3} element={Element.Fire} />
          <ElementMatrixCell amount={3} element={Element.Water} />
          <ElementMatrixCell amount={3} element={Element.Earth} />
          <ElementMatrixCell amount={3} element={Element.Air} />
        </tr>
      </tbody>
    </table>
  );

  function ElementMatrixCell({
    amount,
    element,
  }: ElementMatrixCellProps): React.ReactElement {
    const getCardsWithElements = showInactiveElements
      ? getCardsWithInactiveElements
      : getCardsWithActiveElements;
    const cardsWithElements = getCardsWithElements(cards, amount, element);
    return (
      <td
        className={
          "ElementMatrixCell" +
          (cardsWithElements.length === 0 ? " ElementMatrixCell--empty" : "")
        }
      >
        {cardsWithElements.map(renderCard)}
      </td>
    );
  }

  function renderCard(card: Card): React.ReactElement {
    const isSelected = selectedCard.unwrapOr(null) === card.cardType;
    const isCapturable = selectedCard.match({
      none: () => false,
      some: (selectedCard) =>
        canCapture(gameState, selectedCard, card.cardType),
    });

    return (
      <CardComponent
        card={card}
        isSelected={isSelected}
        isCapturable={isCapturable}
        onCardClicked={onCardClicked}
      />
    );
  }
}
