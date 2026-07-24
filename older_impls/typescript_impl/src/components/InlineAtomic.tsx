import React from "react";
import { cardEmojis } from "../cardMaps";
import { isAnimalStep } from "../gameUtil";
import { Atomic, CardType, PlyType } from "../types";
import "./styles/InlineAtomic.css";

interface Props {
  atomic: Atomic;
  plyNumber: number;
  ellipsis: Ellipsis;
}

export enum Ellipsis {
  Before,
  After,
  None,
}

export default function InlineAtomic({
  atomic,
  plyNumber,
  ellipsis,
}: Props): React.ReactElement {
  const plyMakerEmoji =
    plyNumber % 2 === 0
      ? getEmoji(CardType.AlphaSnipe)
      : getEmoji(CardType.BetaSnipe);

  if (isAnimalStep(atomic)) {
    switch (ellipsis) {
      case Ellipsis.Before:
        return (
          <>
            <div className="InlinePlyNumber">{plyMakerEmoji + plyNumber}.</div>
            {" ..."}
            {getEmoji(atomic.moved)}
            {atomic.destination}
          </>
        );
      case Ellipsis.After:
        return (
          <>
            <div className="InlinePlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
            {getEmoji(atomic.moved)}
            {atomic.destination}
            {", ..."}
          </>
        );
      case Ellipsis.None:
        return (
          <>
            <div className="InlinePlyNumber">{plyMakerEmoji + plyNumber}.</div>{" "}
            {getEmoji(atomic.moved)}
            {atomic.destination}
          </>
        );
    }
  } else {
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
  }
}

function getEmoji(cardType: CardType): string {
  return cardEmojis[cardType];
}
