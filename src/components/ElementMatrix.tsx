import React from "react";
import { Option } from "rusty-ts";
import { CardType, Element, GameState, Card } from "../types";
import CardComponent from "./CardComponent";
import "./styles/ElementMatrix.css";
import { cardProperties } from "../cardMaps";

interface Props {
  gameState: GameState;
  cards: Card[];
  selectedCard: Option<CardType>;
  onCardClicked(card: Card): void;
}

export default function ElementMatrix({
  gameState,
  cards,
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

function getCardsWithElements(
  cards: Card[],
  amount: number,
  element: Element
): Card[] {
  return cards.filter(
    (card) => countElement(card.cardType, element) === amount
  );
}

function countElement(cardType: CardType, element: Element): number {
  const { elements } = cardProperties[cardType];
  return elements.match({
    none: () => 0,
    some: ({ double, single }) => {
      if (double === single) {
        if (double === element) {
          return 3;
        } else {
          return 0;
        }
      }

      if (double === element) {
        return 2;
      }

      if (single === element) {
        return 1;
      }

      return 0;
    },
  });
}
