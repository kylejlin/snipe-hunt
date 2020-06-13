import React from "react";
import { Option } from "rusty-ts";
import { cardEmojis } from "../cardMaps";
import { opponentOf, snipeOf } from "../gameUtil";
import { Atomic, CardType, Player, PlyType } from "../types";
import "./styles/InlineAtomic.css";

interface Props {
  atomic: Atomic;
  isSecondAnimalStep: boolean;
  plyNumber: number;
  winner: Option<Player>;
}

export default function InlineAtomic({
  atomic,
  isSecondAnimalStep,
  plyNumber,
  winner: optWinner,
}: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.AlphaSnipe)
      : getEmoji(CardType.BetaSnipe);

  if ("plyType" in atomic) {
    switch (atomic.plyType) {
      case PlyType.SnipeStep:
        return (
          <>
            {" "}
            <div className="InlinePlyNumber">
              {plyMakerEmoji + plyNumber}.
            </div>{" "}
            {plyMakerEmoji}
            {atomic.destination}
          </>
        );
      case PlyType.Drop:
        return (
          <>
            <div className="InlinePlyNumber">{plyMakerEmoji + plyNumber}.</div>
            {" !"}
            {getEmoji(atomic.dropped)}
            {atomic.destination}
          </>
        );
    }
  } else {
    if (isSecondAnimalStep) {
      return (
        <>
          <div className="InlinePlyNumber">{plyMakerEmoji + plyNumber}.</div>
          {" ..."}
          {getEmoji(atomic.moved)}
          {atomic.destination}
        </>
      );
    } else {
      return (
        <>
          <div className="InlinePlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
          {getEmoji(atomic.moved)}
          {atomic.destination}{" "}
          {optWinner.match({
            some: (winner) => {
              const winnerEmoji = cardEmojis[snipeOf(winner)];
              const loserEmoji = cardEmojis[snipeOf(opponentOf(winner))];
              return "; " + winnerEmoji + ">" + loserEmoji;
            },

            none: () => "; ...",
          })}
        </>
      );
    }
  }
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
