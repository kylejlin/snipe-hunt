import { option } from "rusty-ts";
import { CardPropertyMap, CardType, Element } from "./types";

const cardPropertiesImpl: CardPropertyMap = {
  [CardType.AlphaSnipe]: {
    elements: option.none(),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.BetaSnipe]: {
    elements: option.none(),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },

  [CardType.Mouse]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Earth },
      promoted: { double: Element.Earth, single: Element.Water },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Ox]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Water },
      promoted: { double: Element.Air, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Tiger]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Fire },
      promoted: { double: Element.Fire, single: Element.Air },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Rabbit]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Water },
      promoted: { double: Element.Water, single: Element.Water },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Dragon]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Air },
      promoted: { double: Element.Air, single: Element.Water },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Snake]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Earth },
      promoted: { double: Element.Water, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Horse]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Air },
      promoted: { double: Element.Air, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Ram]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Air },
      promoted: { double: Element.Earth, single: Element.Air },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Monkey]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Earth },
      promoted: { double: Element.Earth, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Rooster]: {
    elements: option.some({
      unpromoted: { double: Element.Air, single: Element.Fire },
      promoted: { double: Element.Fire, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Dog]: {
    elements: option.some({
      unpromoted: { double: Element.Fire, single: Element.Water },
      promoted: { double: Element.Air, single: Element.Air },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Boar]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Fire },
      promoted: { double: Element.Fire, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },

  [CardType.Fish]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Water },
      promoted: { double: Element.Water, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Elephant]: {
    elements: option.some({
      unpromoted: { double: Element.Earth, single: Element.Earth },
      promoted: { double: Element.Fire, single: Element.Water },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
  [CardType.Squid]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Fire },
      promoted: { double: Element.Earth, single: Element.Earth },
    }),
    canUnpromotedMoveBackward: true,
    canPromotedMoveBackward: false,
  },
  [CardType.Frog]: {
    elements: option.some({
      unpromoted: { double: Element.Water, single: Element.Air },
      promoted: { double: Element.Water, single: Element.Fire },
    }),
    canUnpromotedMoveBackward: false,
    canPromotedMoveBackward: false,
  },
};

export default cardPropertiesImpl;
