import React from "react";
import { cardEmojis } from "../cardMaps";
import * as gameUtil from "../gameUtil";
import { Card, Player } from "../types";
import "./styles/CardComponent.css";

interface Props {
  card: Card;
  isSelected: boolean;
  onCardClicked(card: Card): void;
}

export default function CardComponent({
  card,
  isSelected,
  onCardClicked,
}: Props): React.ReactElement {
  return (
    <div
      className={
        "CardComponent" +
        (card.allegiance === Player.Alpha
          ? " CardComponent--alpha"
          : " CardComponent--beta") +
        (isSelected ? " CardComponent--selected" : "") +
        (gameUtil.canRetreat(card.cardType)
          ? " CardComponent--canMoveBackward"
          : "")
      }
      onClick={() => onCardClicked(card)}
    >
      {cardEmojis[card.cardType]}
    </div>
  );
}
