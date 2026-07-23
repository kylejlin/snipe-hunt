public class Row {
    public static final int AlphaReserve = 0;
    public static final int Rank1 = 1;
    public static final int Rank2 = 2;
    public static final int Rank3 = 3;
    public static final int Rank4 = 4;
    public static final int Rank5 = 5;
    public static final int Rank6 = 6;
    public static final int BetaReserve = 7;

    public static final int[] ALL = { 0, 1, 2, 3, 4, 5, 6, 7 };
    public static final int[] RANKS = { 1, 2, 3, 4, 5, 6 };

    public static boolean isReserve(int row) {
        return row == Row.AlphaReserve || row == Row.BetaReserve;
    }
}