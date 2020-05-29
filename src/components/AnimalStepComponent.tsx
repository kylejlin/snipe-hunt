import React from "react";
import { cardEmojis } from "../cardMaps";
import { CardType, AnimalStep } from "../types";
import "./styles/SubPlyComponent.css";

interface Props {
  step: AnimalStep;
  plyNumber: number;
}

export default function AnimalStepComponent({
  step,
  plyNumber,
}: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.AlphaSnipe)
      : getEmoji(CardType.BetaSnipe);
  return (
    <li>
      <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
      {getEmoji(step.moved)}
      {step.destination}
      {"; ..."}
    </li>
  );
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
