public class CardProperties {
    private static final boolean[] CAN_RETREAT = new boolean[34];

    static {
        CAN_RETREAT[CardType.Rat1.raw] = true;
        CAN_RETREAT[CardType.Rabbit1.raw] = true;
        CAN_RETREAT[CardType.Snake1.raw] = true;
        CAN_RETREAT[CardType.Ram1.raw] = true;
        CAN_RETREAT[CardType.Boar1.raw] = true;
        CAN_RETREAT[CardType.Squid1.raw] = true;

        CAN_RETREAT[CardType.Rat2.raw] = true;
        CAN_RETREAT[CardType.Rabbit2.raw] = true;
        CAN_RETREAT[CardType.Snake2.raw] = true;
        CAN_RETREAT[CardType.Ram2.raw] = true;
        CAN_RETREAT[CardType.Boar2.raw] = true;
        CAN_RETREAT[CardType.Squid2.raw] = true;

        CAN_RETREAT[CardType.AlphaSnipe.raw] = true;
        CAN_RETREAT[CardType.BetaSnipe.raw] = true;
    }

    public static boolean canRetreat(CardType c) {
        return CAN_RETREAT[c.raw];
    }

    private static final int EmptySet = 0;
    private static final int F1 = 1 << 0;
    private static final int F2 = 1 << 1;
    private static final int F3 = 1 << 2;
    private static final int W1 = 1 << 3;
    private static final int W2 = 1 << 4;
    private static final int W3 = 1 << 5;
    private static final int E1 = 1 << 6;
    private static final int E2 = 1 << 7;
    private static final int E3 = 1 << 8;
    private static final int A1 = 1 << 9;
    private static final int A2 = 1 << 10;
    private static final int A3 = 1 << 11;

    private static final int[] ELEMENT_COUNTS = new int[34];

    static {
        ELEMENT_COUNTS[CardType.Rat1.raw] = F2 | E1;
        ELEMENT_COUNTS[CardType.Ox1.raw] = E2 | W1;
        ELEMENT_COUNTS[CardType.Tiger1.raw] = F3;
        ELEMENT_COUNTS[CardType.Rabbit1.raw] = A2 | W1;
        ELEMENT_COUNTS[CardType.Dragon1.raw] = A3;
        ELEMENT_COUNTS[CardType.Snake1.raw] = W2 | E1;
        ELEMENT_COUNTS[CardType.Horse1.raw] = F2 | A1;
        ELEMENT_COUNTS[CardType.Ram1.raw] = E2 | A1;
        ELEMENT_COUNTS[CardType.Monkey1.raw] = A2 | E1;
        ELEMENT_COUNTS[CardType.Rooster1.raw] = A2 | F1;
        ELEMENT_COUNTS[CardType.Dog1.raw] = F2 | W1;
        ELEMENT_COUNTS[CardType.Boar1.raw] = E2 | F1;

        ELEMENT_COUNTS[CardType.Fish1.raw] = W3;
        ELEMENT_COUNTS[CardType.Elephant1.raw] = E3;
        ELEMENT_COUNTS[CardType.Squid1.raw] = W2 | F1;
        ELEMENT_COUNTS[CardType.Frog1.raw] = W2 | A1;

        ELEMENT_COUNTS[CardType.Rat2.raw] = F2 | E1;
        ELEMENT_COUNTS[CardType.Ox2.raw] = E2 | W1;
        ELEMENT_COUNTS[CardType.Tiger2.raw] = F3;
        ELEMENT_COUNTS[CardType.Rabbit2.raw] = A2 | W1;
        ELEMENT_COUNTS[CardType.Dragon2.raw] = A3;
        ELEMENT_COUNTS[CardType.Snake2.raw] = W2 | E1;
        ELEMENT_COUNTS[CardType.Horse2.raw] = F2 | A1;
        ELEMENT_COUNTS[CardType.Ram2.raw] = E2 | A1;
        ELEMENT_COUNTS[CardType.Monkey2.raw] = A2 | E1;
        ELEMENT_COUNTS[CardType.Rooster2.raw] = A2 | F1;
        ELEMENT_COUNTS[CardType.Dog2.raw] = F2 | W1;
        ELEMENT_COUNTS[CardType.Boar2.raw] = E2 | F1;

        ELEMENT_COUNTS[CardType.Fish2.raw] = W3;
        ELEMENT_COUNTS[CardType.Elephant2.raw] = E3;
        ELEMENT_COUNTS[CardType.Squid2.raw] = W2 | F1;
        ELEMENT_COUNTS[CardType.Frog2.raw] = W2 | A1;

        ELEMENT_COUNTS[CardType.AlphaSnipe.raw] = EmptySet;
        ELEMENT_COUNTS[CardType.BetaSnipe.raw] = EmptySet;
    }

    public static boolean doesStepActivateTriplet(CardType newCardType, Iterable<Card> oldCards) {
        int newCardElementCount = ELEMENT_COUNTS[newCardType.raw];

        int union = newCardElementCount;
        for (Card card : oldCards) {
            union |= ELEMENT_COUNTS[card.type.raw];
        }

        int fireShift = (newCardElementCount & (0b111 << 0)) != 0 ? 0 : 12;
        int waterShift = (newCardElementCount & (0b111 << 3)) != 0 ? 3 : 12;
        int earthShift = (newCardElementCount & (0b111 << 6)) != 0 ? 6 : 12;
        int airShift = (newCardElementCount & (0b111 << 9)) != 0 ? 9 : 12;

        return ((union >>> fireShift) & 0b111) == 0b111 || ((union >>> waterShift) & 0b111) == 0b111
                || ((union >>> earthShift) & 0b111) == 0b111 || ((union >>> airShift) & 0b111) == 0b111;
    }
}
