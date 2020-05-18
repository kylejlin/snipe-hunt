import React from "react";
import { cardEmojis } from "../cardMaps";
import { CardType, SubPly, PlyType, SubPlyType } from "../types";
import "./styles/SubPlyComponent.css";

interface Props {
  subPly: SubPly;
  plyNumber: number;
}

export default function SubPlyComponent({
  subPly,
  plyNumber,
}: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.AlphaSnipe)
      : getEmoji(CardType.BetaSnipe);
  switch (subPly.subPlyType) {
    case SubPlyType.Demote:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div> -
          {getEmoji(subPly.demoted)}; ...
        </li>
      );
    case SubPlyType.Move:
      return (
        <li>
          <div className="PlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
          {getEmoji(subPly.moved)}
          {subPly.destination}
          {subPly.captures.map(getEmoji)}; ...
        </li>
      );
  }
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
