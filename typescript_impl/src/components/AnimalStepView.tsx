import React from "react";
import { Option } from "rusty-ts";
import { cardEmojis } from "../cardMaps";
import * as gameUtil from "../gameUtil";
import { AnimalStep, CardType, Player } from "../types";
import "./styles/AnimalStepView.css";

interface Props {
  step: AnimalStep;
  plyNumber: number;
  winner: Option<Player>;
}

export default function AnimalStepView({
  step,
  plyNumber,
  winner: optWinner,
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
      {optWinner.match({
        some: (winner) => {
          const winnerEmoji = cardEmojis[gameUtil.snipeOf(winner)];
          const loserEmoji =
            cardEmojis[gameUtil.snipeOf(gameUtil.opponentOf(winner))];
          return ", " + winnerEmoji + ">" + loserEmoji;
        },

        none: () => ", ...",
      })}
    </li>
  );
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
