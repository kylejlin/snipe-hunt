import javafx.scene.image.Image;

public class Images {
    private static Image getImage(String path) {
        return new Image(Images.class.getClassLoader().getResource(path).toString());
    }

    public static final Image ALPHA_BACKGROUND = getImage("resources/AlphaBackground.png");
    public static final Image ALPHA_RETREATER_BACKGROUND = getImage("resources/AlphaRetreaterBackground.png");
    public static final Image BETA_BACKGROUND = getImage("resources/BetaBackground.png");
    public static final Image BETA_RETREATER_BACKGROUND = getImage("resources/BetaRetreaterBackground.png");

    public static final Image SELECTION_OVERLAY = getImage("resources/SelectionOverlay.png");

    public static final Image[] FOREGROUNDS = { getImage("resources/Rat.png"), getImage("resources/Ox.png"),
            getImage("resources/Tiger.png"), getImage("resources/Rabbit.png"), getImage("resources/Dragon.png"),
            getImage("resources/Snake.png"), getImage("resources/Horse.png"), getImage("resources/Ram.png"),
            getImage("resources/Monkey.png"), getImage("resources/Rooster.png"), getImage("resources/Dog.png"),
            getImage("resources/Boar.png"), getImage("resources/Fish.png"), getImage("resources/Elephant.png"),
            getImage("resources/Squid.png"), getImage("resources/Frog.png"),

            null, null, null, null, null, null, null, null, null, null, null, null, null, null, null, null,

            getImage("resources/AlphaSnipe.png"), getImage("resources/BetaSnipe.png"), };

    static {
        for (int i = 16; i <= 31; i++) {
            FOREGROUNDS[i] = FOREGROUNDS[i - 16];
        }
    }

    public static final Image[] ROW_MARKERS = { getImage("resources/AlphaReserveMarker.png"),
            getImage("resources/Rank1Marker.png"), getImage("resources/Rank2Marker.png"),
            getImage("resources/Rank3Marker.png"), getImage("resources/Rank4Marker.png"),
            getImage("resources/Rank5Marker.png"), getImage("resources/Rank6Marker.png"),
            getImage("resources/BetaReserveMarker.png"), };
}