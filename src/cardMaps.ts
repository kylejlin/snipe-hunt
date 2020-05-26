import { option } from "rusty-ts";
import { CardType, Element, CardMap, CardProperties } from "./types";

export const cardProperties: CardMap<CardProperties> = {
  [CardType.Mouse1]: {
    elements: option.some({ double: Element.Fire, single: Element.Earth }),
    canRetreat: true,
  },
  [CardType.Ox1]: {
    elements: option.some({ double: Element.Earth, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Tiger1]: {
    elements: option.some({ double: Element.Fire, single: Element.Fire }),
    canRetreat: false,
  },
  [CardType.Rabbit1]: {
    elements: option.some({ double: Element.Air, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Dragon1]: {
    elements: option.some({ double: Element.Air, single: Element.Air }),
    canRetreat: false,
  },
  [CardType.Snake1]: {
    elements: option.some({ double: Element.Water, single: Element.Earth }),
    canRetreat: true,
  },
  [CardType.Horse1]: {
    elements: option.some({ double: Element.Fire, single: Element.Air }),
    canRetreat: false,
  },
  [CardType.Ram1]: {
    elements: option.some({ double: Element.Earth, single: Element.Air }),
    canRetreat: true,
  },
  [CardType.Monkey1]: {
    elements: option.some({ double: Element.Air, single: Element.Earth }),
    canRetreat: false,
  },
  [CardType.Rooster1]: {
    elements: option.some({ double: Element.Air, single: Element.Fire }),
    canRetreat: false,
  },
  [CardType.Dog1]: {
    elements: option.some({ double: Element.Fire, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Boar1]: {
    elements: option.some({ double: Element.Earth, single: Element.Fire }),
    canRetreat: true,
  },

  [CardType.Fish1]: {
    elements: option.some({ double: Element.Water, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Elephant1]: {
    elements: option.some({ double: Element.Earth, single: Element.Earth }),
    canRetreat: false,
  },
  [CardType.Squid1]: {
    elements: option.some({ double: Element.Water, single: Element.Fire }),
    canRetreat: true,
  },
  [CardType.Frog1]: {
    elements: option.some({ double: Element.Water, single: Element.Air }),
    canRetreat: false,
  },

  [CardType.Mouse2]: {
    elements: option.some({ double: Element.Fire, single: Element.Earth }),
    canRetreat: true,
  },
  [CardType.Ox2]: {
    elements: option.some({ double: Element.Earth, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Tiger2]: {
    elements: option.some({ double: Element.Fire, single: Element.Fire }),
    canRetreat: false,
  },
  [CardType.Rabbit2]: {
    elements: option.some({ double: Element.Air, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Dragon2]: {
    elements: option.some({ double: Element.Air, single: Element.Air }),
    canRetreat: false,
  },
  [CardType.Snake2]: {
    elements: option.some({ double: Element.Water, single: Element.Earth }),
    canRetreat: true,
  },
  [CardType.Horse2]: {
    elements: option.some({ double: Element.Fire, single: Element.Air }),
    canRetreat: false,
  },
  [CardType.Ram2]: {
    elements: option.some({ double: Element.Earth, single: Element.Air }),
    canRetreat: true,
  },
  [CardType.Monkey2]: {
    elements: option.some({ double: Element.Air, single: Element.Earth }),
    canRetreat: false,
  },
  [CardType.Rooster2]: {
    elements: option.some({ double: Element.Air, single: Element.Fire }),
    canRetreat: false,
  },
  [CardType.Dog2]: {
    elements: option.some({ double: Element.Fire, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Boar2]: {
    elements: option.some({ double: Element.Earth, single: Element.Fire }),
    canRetreat: true,
  },

  [CardType.Fish2]: {
    elements: option.some({ double: Element.Water, single: Element.Water }),
    canRetreat: false,
  },
  [CardType.Elephant2]: {
    elements: option.some({ double: Element.Earth, single: Element.Earth }),
    canRetreat: false,
  },
  [CardType.Squid2]: {
    elements: option.some({ double: Element.Water, single: Element.Fire }),
    canRetreat: true,
  },
  [CardType.Frog2]: {
    elements: option.some({ double: Element.Water, single: Element.Air }),
    canRetreat: false,
  },

  [CardType.AlphaSnipe]: {
    elements: option.none(),
    canRetreat: true,
  },
  [CardType.BetaSnipe]: {
    elements: option.none(),
    canRetreat: true,
  },
};

export const cardEmojis: CardMap<string> = {
  [CardType.Mouse1]: "🐀",
  [CardType.Mouse2]: "🐀",
  [CardType.Ox1]: "🐮",
  [CardType.Ox2]: "🐮",
  [CardType.Tiger1]: "🐯",
  [CardType.Tiger2]: "🐯",
  [CardType.Rabbit1]: "🐇",
  [CardType.Rabbit2]: "🐇",
  [CardType.Dragon1]: "🐉 ",
  [CardType.Dragon2]: "🐉 ",
  [CardType.Snake1]: "🐍",
  [CardType.Snake2]: "🐍",
  [CardType.Horse1]: "🐴",
  [CardType.Horse2]: "🐴",
  [CardType.Ram1]: "🐏",
  [CardType.Ram2]: "🐏",
  [CardType.Monkey1]: "🐵",
  [CardType.Monkey2]: "🐵",
  [CardType.Rooster1]: "🐓",
  [CardType.Rooster2]: "🐓",
  [CardType.Dog1]: "🐶",
  [CardType.Dog2]: "🐶",
  [CardType.Boar1]: "🐗",
  [CardType.Boar2]: "🐗",
  [CardType.Fish1]: "🐟",
  [CardType.Fish2]: "🐟",
  [CardType.Elephant1]: "🐘",
  [CardType.Elephant2]: "🐘",
  [CardType.Squid1]: "🦑",
  [CardType.Squid2]: "🦑",
  [CardType.Frog1]: "🐸",
  [CardType.Frog2]: "🐸",

  [CardType.AlphaSnipe]: "α",
  [CardType.BetaSnipe]: "β",
};
