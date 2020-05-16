import React from "react";
import { Card, CardType } from "../types";

interface Props {
  card: Card;
  isSelected: boolean;
  onSelect(): void;
}

export default function CardComponent({
  card,
  isSelected,
  onSelect,
}: Props): React.ReactElement {
  return (
    <div
      className={
        "CardComponent" +
        (isSelected ? " CardComponent--selected" : "") +
        (card.isPromoted ? " CardComponent--promoted" : "")
      }
      onClick={onSelect}
    >
      {getCardDisplayName(card)}
    </div>
  );
}
function getCardDisplayName(card: Card): string {
  const { cardType } = card;
  if (cardType === CardType.AlphaSnipe || cardType === CardType.BetaSnipe) {
    return "Snipe";
  } else {
    return CardType[cardType];
  }
}
