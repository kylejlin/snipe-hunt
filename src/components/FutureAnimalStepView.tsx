import React from "react";
import { Option } from "rusty-ts";
import { cardEmojis } from "../cardMaps";
import * as gameUtil from "../gameUtil";
import { AnimalStep, CardType, Player } from "../types";
import "./styles/SubPlyComponent.css";

interface Props {
  step: AnimalStep;
  plyNumber: number;
}

export default function FutureAnimalStepView({
  step,
  plyNumber,
}: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.AlphaSnipe)
      : getEmoji(CardType.BetaSnipe);
  return (
    <li>
      <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>
      {" ..."}
      {getEmoji(step.moved)}
      {step.destination}
    </li>
  );
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
