import React from "react";
import { cardEmojis } from "../cardMaps";
import {
  getCardsWithActiveElements,
  getCardsWithInactiveElements,
} from "../game";
import { Card, Element, Player } from "../types";
import "./styles/ElementMatrix.css";

interface Props {
  cards: Card[];
  showInactiveElements: boolean;
}

export default function ElementMatrix({
  cards,
  showInactiveElements,
}: Props): React.ReactElement {
  const getCardsWithElements = showInactiveElements
    ? getCardsWithInactiveElements
    : getCardsWithActiveElements;
  return (
    <table className="ElementMatrix">
      <tbody>
        <tr>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 1, Element.Fire).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 1, Element.Water).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 1, Element.Earth).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 1, Element.Air).map(renderCard)}
          </td>
        </tr>
        <tr>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 2, Element.Fire).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 2, Element.Water).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 2, Element.Earth).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 2, Element.Air).map(renderCard)}
          </td>
        </tr>
        <tr>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 3, Element.Fire).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 3, Element.Water).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 3, Element.Earth).map(renderCard)}
          </td>
          <td className="ElementMatrixCell">
            {getCardsWithElements(cards, 3, Element.Air).map(renderCard)}
          </td>
        </tr>
      </tbody>
    </table>
  );
}

function renderCard(card: Card): React.ReactElement {
  return (
    <div
      className={
        "CardComponent" +
        (card.allegiance === Player.Alpha
          ? " CardComponent--alpha"
          : " CardComponent--beta") +
        (card.isPromoted ? " CardComponent--promoted" : "")
      }
    >
      {cardEmojis[card.cardType]}
    </div>
  );
}
