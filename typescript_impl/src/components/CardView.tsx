import React from "react";
import { cardEmojis } from "../cardMaps";
import * as gameUtil from "../gameUtil";
import { Card, Player } from "../types";
import "./styles/CardView.css";

interface Props {
  card: Card;
  isSelected: boolean;
  onCardClicked(card: Card): void;
}

export default function CardView({
  card,
  isSelected,
  onCardClicked,
}: Props): React.ReactElement {
  return (
    <div
      className={
        "CardView" +
        (card.allegiance === Player.Alpha
          ? " CardView--alpha"
          : " CardView--beta") +
        (isSelected ? " CardView--selected" : "") +
        (gameUtil.canRetreat(card.cardType) ? " CardView--canMoveBackward" : "")
      }
      onClick={() => onCardClicked(card)}
    >
      {cardEmojis[card.cardType]}
    </div>
  );
}
