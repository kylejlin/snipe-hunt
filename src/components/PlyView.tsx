import React from "react";
import { cardEmojis } from "../cardMaps";
import { CardType, Ply, PlyType } from "../types";
import "./styles/PlyView.css";

interface Props {
  ply: Ply;
  plyNumber: number;
}

export default function PlyView({ ply, plyNumber }: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.AlphaSnipe)
      : getEmoji(CardType.BetaSnipe);
  switch (ply.plyType) {
    case PlyType.SnipeStep:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
          {plyMakerEmoji}
          {ply.destination}
        </li>
      );
    case PlyType.Drop:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>
          {" !"}
          {getEmoji(ply.dropped)}
          {ply.destination}
        </li>
      );
    case PlyType.TwoAnimalSteps:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
          {getEmoji(ply.first.moved)}
          {ply.first.destination}
          {"; "}
          {getEmoji(ply.second.moved)}
          {ply.second.destination}
        </li>
      );
  }
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
