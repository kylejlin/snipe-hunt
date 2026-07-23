public enum CardType {
    Rat1(0), Ox1(1), Tiger1(2), Rabbit1(3), Dragon1(4), Snake1(5), Horse1(6), Ram1(7), Monkey1(8), Rooster1(9),
    Dog1(10), Boar1(11),

    Fish1(12), Elephant1(13), Squid1(14), Frog1(15),

    Rat2(16), Ox2(17), Tiger2(18), Rabbit2(19), Dragon2(20), Snake2(21), Horse2(22), Ram2(23), Monkey2(24),
    Rooster2(25), Dog2(26), Boar2(27),

    Fish2(28), Elephant2(29), Squid2(30), Frog2(31),

    AlphaSnipe(32), BetaSnipe(33);

    public final int raw;

    CardType(int raw) {
        this.raw = raw;
    }

    public boolean isSnipe() {
        return this == CardType.AlphaSnipe || this == CardType.BetaSnipe;
    }

    public boolean canRetreat() {
        return CardProperties.canRetreat(this);
    }

    public String toUnnumberedString() {
        String s = this.toString();
        if (Character.isDigit(s.charAt(s.length() - 1))) {
            return s.substring(0, s.length() - 1);
        } else {
            return s;
        }
    }
}