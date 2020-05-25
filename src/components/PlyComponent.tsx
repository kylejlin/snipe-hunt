import React from "react";
import { cardEmojis } from "../cardMaps";
import { CardType, Ply, PlyType } from "../types";
import "./styles/PlyComponent.css";

interface Props {
  ply: Ply;
  plyNumber: number;
}

export default function PlyComponent({
  ply,
  plyNumber,
}: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.Snipe)
      : getEmoji(CardType.BetaSnipe);
  switch (ply.plyType) {
    case PlyType.DemoteMove:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div> -
          {getEmoji(ply.demoted)}; {getEmoji(ply.moved)}
          {ply.destination}
          {ply.captures.map(getEmoji)}
        </li>
      );
    case PlyType.MovePromote:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
          {getEmoji(ply.moved)}
          {ply.destination}
          {ply.captures.map(getEmoji)}; +{getEmoji(ply.promoted)}
        </li>
      );
    case PlyType.Drop:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div> !
          {getEmoji(ply.dropped)}
        </li>
      );
  }
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
