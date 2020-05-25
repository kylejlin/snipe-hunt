import { option } from "rusty-ts";
import { CardType, Element, CardMap, CardProperties } from "./types";

export const cardProperties: CardMap<CardProperties> = {
  [CardType.AlphaSnipe]: {
    elements: option.none(),
    canRetreat: true,
  },
  [CardType.BetaSnipe]: {
    elements: option.none(),
    canRetreat: true,
  },

  [CardType.Mouse]: {
    elements: option.some({ double: Element.Fire, single: Element.Earth }),
    canRetreat: true,
  },
  [CardType.Ox]: {
    elements: option.some({ double: Element.Earth, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Tiger]: {
    elements: option.some({ double: Element.Fire, single: Element.Fire }),
    canRetreat: false,
  },
  [CardType.Rabbit]: {
    elements: option.some({ double: Element.Air, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Dragon]: {
    elements: option.some({ double: Element.Air, single: Element.Air }),
    canRetreat: false,
  },
  [CardType.Snake]: {
    elements: option.some({ double: Element.Water, single: Element.Earth }),
    canRetreat: true,
  },
  [CardType.Horse]: {
    elements: option.some({ double: Element.Fire, single: Element.Air }),
    canRetreat: false,
  },
  [CardType.Ram]: {
    elements: option.some({ double: Element.Earth, single: Element.Air }),
    canRetreat: true,
  },
  [CardType.Monkey]: {
    elements: option.some({ double: Element.Air, single: Element.Earth }),
    canRetreat: false,
  },
  [CardType.Rooster]: {
    elements: option.some({ double: Element.Air, single: Element.Fire }),
    canRetreat: false,
  },
  [CardType.Dog]: {
    elements: option.some({ double: Element.Fire, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Boar]: {
    elements: option.some({ double: Element.Earth, single: Element.Fire }),
    canRetreat: true,
  },

  [CardType.Fish]: {
    elements: option.some({ double: Element.Water, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Elephant]: {
    elements: option.some({ double: Element.Earth, single: Element.Earth }),
    canRetreat: false,
  },
  [CardType.Squid]: {
    elements: option.some({ double: Element.Water, single: Element.Fire }),
    canRetreat: true,
  },
  [CardType.Frog]: {
    elements: option.some({ double: Element.Water, single: Element.Air }),
    canRetreat: false,
  },
};

export const cardEmojis: CardMap<string> = {
  [CardType.AlphaSnipe]: "α",
  [CardType.BetaSnipe]: "β",
  [CardType.Mouse]: "🐀",
  [CardType.Ox]: "🐮",
  [CardType.Tiger]: "🐯",
  [CardType.Rabbit]: "🐇",
  [CardType.Dragon]: "🐉 ",
  [CardType.Snake]: "🐍",
  [CardType.Horse]: "🐴",
  [CardType.Ram]: "🐏",
  [CardType.Monkey]: "🐵",
  [CardType.Rooster]: "🐓",
  [CardType.Dog]: "🐶",
  [CardType.Boar]: "🐗",
  [CardType.Fish]: "🐟",
  [CardType.Elephant]: "🐘",
  [CardType.Squid]: "🦑",
  [CardType.Frog]: "🐸",
};
