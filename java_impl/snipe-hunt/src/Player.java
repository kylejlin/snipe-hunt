public enum Player {
    Alpha(0), Beta(1);

    public final int raw;

    Player(int raw) {
        this.raw = raw;
    }

    public Player opponent() {
        if (this == Player.Alpha) {
            return Player.Beta;
        } else {
            return Player.Alpha;
        }
    }

    public String getGreekLetter() {
        if (this == Player.Alpha) {
            return "α";
        } else {
            return "β";
        }
    }
}