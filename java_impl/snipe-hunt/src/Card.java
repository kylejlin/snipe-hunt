import javafx.scene.image.Image;

public class Card {
    public CardType type;
    public Player allegiance;

    public Card(CardType type, Player allegiance) {
        this.type = type;
        this.allegiance = allegiance;
    }

    public Image[] getImages() {
        Image background;

        if (allegiance == Player.Alpha) {
            if (canRetreat()) {
                background = Images.ALPHA_RETREATER_BACKGROUND;
            } else {
                background = Images.ALPHA_BACKGROUND;
            }
        } else {
            if (canRetreat()) {
                background = Images.BETA_RETREATER_BACKGROUND;
            } else {
                background = Images.BETA_BACKGROUND;
            }
        }

        Image foreground = Images.FOREGROUNDS[type.raw];

        return new Image[] { background, foreground };
    }

    public boolean canRetreat() {
        return CardProperties.canRetreat(type);
    }

    public Card safeClone() {
        return new Card(type, allegiance);
    }
}