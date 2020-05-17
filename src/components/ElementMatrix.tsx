import React from "react";
import { Option } from "rusty-ts";
import { cardEmojis } from "../cardMaps";
import {
  getCardsWithActiveElements,
  getCardsWithInactiveElements,
  canMoveBackward,
} from "../game";
import { Card, CardType, Element, Player } from "../types";
import "./styles/ElementMatrix.css";
import CardComponent from "./CardComponent";

interface Props {
  cards: Card[];
  showInactiveElements: boolean;
  selectedCard: Option<CardType>;
  onCardClicked(card: Card): void;
}

export default function ElementMatrix({
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

    return (
      <CardComponent
        card={card}
        isSelected={isSelected}
        onCardClicked={onCardClicked}
      />
    );
  }
}
