import java.util.ArrayList;
import java.util.Optional;

import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Node;
import javafx.scene.control.Button;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.layout.FlowPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.StackPane;
import javafx.scene.text.Font;
import javafx.scene.text.FontPosture;
import javafx.scene.text.Text;

public class Page {
    private static final String PAGE_DELIMITER = "\n\n---";
    private static final String PARAGRAPH_DELIMITER = "\n\n";

    public ArrayList<Paragraph> paragraphs;

    /**
     * Parses a markdown file that specifies a list of pages.
     * 
     * @param src A scanner that consumes the markdown file.
     * @return A list of the pages specified by the markdown file.
     */
    public static ArrayList<Page> parse(String src, Runnable onNextButtonClicked) {
        ArrayList<Page> pages = new ArrayList<>();

        ArrayList<String> pageSrcs = getPageSrcs(src);

        for (String pageSrc : pageSrcs) {
            ArrayList<Paragraph> paragraphs = new ArrayList<>();

            ArrayList<String> paragraphSrcs = getPageParagraphSrcs(pageSrc);
            for (String paragraphSrc : paragraphSrcs) {
                paragraphs.add(ParagraphParser.parse(paragraphSrc, onNextButtonClicked));
            }

            pages.add(new Page(paragraphs));
        }

        return pages;
    }

    private static ArrayList<String> getPageSrcs(String src) {
        ArrayList<String> srcs = new ArrayList<>();

        int i = 0;
        int j = src.indexOf(PAGE_DELIMITER, i);
        while (j != -1) {
            srcs.add(src.substring(i, j));
            i = j + PAGE_DELIMITER.length();
            j = src.indexOf(PAGE_DELIMITER, i);
        }

        return srcs;
    }

    private static ArrayList<String> getPageParagraphSrcs(String src) {
        ArrayList<String> srcs = new ArrayList<>();

        int i = 0;
        int j = src.indexOf(PARAGRAPH_DELIMITER, i);
        while (j != -1) {
            srcs.add(src.substring(i, j));
            i = j + PARAGRAPH_DELIMITER.length();
            j = src.indexOf(PARAGRAPH_DELIMITER, i);
        }

        if (i < src.length()) {
            srcs.add(src.substring(i));
        }

        return srcs;
    }

    private Page(ArrayList<Paragraph> paragraphs) {
        this.paragraphs = paragraphs;
    }

    public Action getExpectedAction() {
        Optional<Action> optUserAction = paragraphs.stream().filter(p -> p instanceof AwaitActionParagraph).findFirst()
                .map(paragraph -> ((AwaitActionParagraph) paragraph).getExpectedAction());
        return optUserAction.orElse(null);
    }

    public Action getExpectedLegalAction() {
        Optional<Action> optUserAction = paragraphs.stream()
                .filter(p -> p instanceof AwaitActionParagraph && ((AwaitActionParagraph) p).isLegal()).findFirst()
                .map(paragraph -> ((AwaitActionParagraph) paragraph).getExpectedAction());
        return optUserAction.orElse(null);
    }

    public Action getPerformedAction() {
        Optional<Action> optUserAction = paragraphs.stream().filter(p -> p instanceof PerformActionParagraph)
                .findFirst().map(paragraph -> ((PerformActionParagraph) paragraph).getPerformedAction());
        return optUserAction.orElse(null);
    }

    public ArrayList<Node> render() {
        ArrayList<Node> nodes = new ArrayList<>();
        for (Paragraph paragraph : paragraphs) {
            nodes.addAll(paragraph.render());
        }
        return nodes;
    }
}

interface Paragraph {
    ArrayList<Node> render();

}

class VisibleParagraph implements Paragraph {
    public static VisibleParagraph parse(String src) {
        ArrayList<Chunk> chunks = new ArrayList<>();

        int i = 0;
        while (i < src.length()) {
            int underscoreIndex = src.indexOf("_", i);
            int bracketIndex = src.indexOf("[", i);

            if ((underscoreIndex != -1 && underscoreIndex < bracketIndex)
                    || (underscoreIndex != -1 && bracketIndex == -1)) {
                if (underscoreIndex > i) {
                    chunks.add(new PlainTextChunk(src.substring(i, underscoreIndex)));
                }

                i = underscoreIndex + "_".length();
                int secondUnderscoreIndex = src.indexOf("_", i);
                if (secondUnderscoreIndex == -1) {
                    throw new IllegalArgumentException(
                            "Illegal visibleParagraphSrc contains unmatched underscore: " + src);
                }
                i = secondUnderscoreIndex + "_".length();
                String italicized = src.substring(underscoreIndex + 1, secondUnderscoreIndex);
                chunks.add(new ItalicizedTextChunk(italicized));
                continue;
            }

            if ((bracketIndex != -1 && bracketIndex < underscoreIndex)
                    || (bracketIndex != -1 && underscoreIndex == -1)) {
                if (bracketIndex > i) {
                    chunks.add(new PlainTextChunk(src.substring(i, bracketIndex)));
                }

                i = bracketIndex + "[".length();
                int secondBracketIndex = src.indexOf("]", i);
                if (secondBracketIndex == -1) {
                    throw new IllegalArgumentException(
                            "Illegal visibleParagraphSrc contains unmatched square brackets: " + src);
                }
                i = secondBracketIndex + "]".length();
                String bracketed = src.substring(bracketIndex, i);
                chunks.add(new ImageChunk(bracketed));
                continue;
            }

            // At this point, underscoreIndex and bracketIndex should both be -1.
            break;
        }

        if (i < src.length()) {
            chunks.add(new PlainTextChunk(src.substring(i)));
        }

        return new VisibleParagraph(chunks);
    }

    private ArrayList<Chunk> chunks;

    public VisibleParagraph(ArrayList<Chunk> chunks) {
        this.chunks = chunks;
    }

    @Override
    public ArrayList<Node> render() {
        FlowPane container = new FlowPane();
        container.setPadding(new Insets(6, 3, 6, 3));
        container.setAlignment(Pos.CENTER_LEFT);
        container.setPrefWrapLength(PassAndPlayScreen.INSTRUCTIONS_BOX_WIDTH);
        container.setMaxWidth(PassAndPlayScreen.INSTRUCTIONS_BOX_WIDTH);

        for (Chunk chunk : chunks) {
            container.getChildren().addAll(chunk.render());
        }

        ArrayList<Node> nodes = new ArrayList<>();
        nodes.add(container);
        return nodes;
    }

}

interface Chunk {
    public static final double FONT_SIZE = 23;
    public static final String FONT_FAMILY = "Arial";

    ArrayList<Node> render();
}

class PlainTextChunk implements Chunk {
    private String str;

    public PlainTextChunk(String str) {
        this.str = str;
    }

    @Override
    public ArrayList<Node> render() {
        ArrayList<String> words = StringUtil.getWordsAndSpaces(str);
        ArrayList<Node> nodes = new ArrayList<>();

        for (String word : words) {
            Text text = new Text("\n".equals(word) ? " " : word);
            text.setFont(Font.font(Chunk.FONT_FAMILY, Chunk.FONT_SIZE));
            nodes.add(text);
        }

        return nodes;
    }
}

class ItalicizedTextChunk implements Chunk {
    private String str;

    public ItalicizedTextChunk(String str) {
        this.str = str;
    }

    @Override
    public ArrayList<Node> render() {
        ArrayList<String> words = StringUtil.getWordsAndSpaces(str);
        ArrayList<Node> nodes = new ArrayList<>();

        for (String word : words) {
            Text text = new Text("\n".equals(word) ? " " : word);
            text.setFont(Font.font(Chunk.FONT_FAMILY, FontPosture.ITALIC, Chunk.FONT_SIZE));
            nodes.add(text);
        }

        return nodes;
    }
}

class ImageChunk implements Chunk {
    private static final double CARD_HEIGHT = 75;

    private String leftCaption;
    private StackPane imageStackPane;
    private String rightCaption;

    public ImageChunk(String bracketedSrc) {
        leftCaption = getLeftCaption(bracketedSrc);
        imageStackPane = getImages(bracketedSrc);
        rightCaption = getRightCaption(bracketedSrc);
    }

    private static String getLeftCaption(String bracketedSrc) {
        switch (bracketedSrc) {
            case "[alpha rat]":
                return "rat (";
            case "[alpha ox]":
                return "ox (";
            case "[alpha tiger]":
                return "tiger (";
            case "[alpha rabbit]":
                return "rabbit (";
            case "[alpha dragon]":
                return "dragon (";
            case "[alpha snake]":
                return "snake (";
            case "[alpha horse]":
                return "horse (";
            case "[alpha ram]":
                return "ram (";
            case "[alpha monkey]":
                return "monkey (";
            case "[alpha rooster]":
                return "rooster (";
            case "[alpha dog]":
                return "dog (";
            case "[alpha boar]":
                return "boar (";

            case "[alpha fish]":
                return "fish (";
            case "[alpha elephant]":
                return "elephant (";
            case "[alpha squid]":
                return "squid (";
            case "[alpha frog]":
                return "frog (";

            case "[beta rat]":
                return "rat (";
            case "[beta ox]":
                return "ox (";
            case "[beta tiger]":
                return "tiger (";
            case "[beta rabbit]":
                return "rabbit (";
            case "[beta dragon]":
                return "dragon (";
            case "[beta snake]":
                return "snake (";
            case "[beta horse]":
                return "horse (";
            case "[beta ram]":
                return "ram (";
            case "[beta monkey]":
                return "monkey (";
            case "[beta rooster]":
                return "rooster (";
            case "[beta dog]":
                return "dog (";
            case "[beta boar]":
                return "boar (";

            case "[beta fish]":
                return "fish (";
            case "[beta elephant]":
                return "elephant (";
            case "[beta squid]":
                return "squid (";
            case "[beta frog]":
                return "frog (";

            case "[alpha snipe]":
                return "alpha snipe (";
            case "[beta snipe]":
                return "beta snipe (";

            case "[UNCAPTIONED alpha reserve]":
                return "";
            case "[rank 1]":
                return "rank 1 (";
            case "[rank 2]":
                return "rank 2 (";
            case "[rank 3]":
                return "rank 3 (";
            case "[rank 4]":
                return "rank 4 (";
            case "[rank 5]":
                return "rank 5 (";
            case "[rank 6]":
                return "rank 6 (";
            case "[UNCAPTIONED beta reserve]":
                return "";

            default:
                throw new IllegalArgumentException("Cannot parse image type: " + bracketedSrc);
        }
    }

    private static StackPane getImages(String bracketedSrc) {
        switch (bracketedSrc) {
            case "[alpha rat]":
                return getCardImageStackPane(CardType.Rat1, Player.Alpha);
            case "[alpha ox]":
                return getCardImageStackPane(CardType.Ox1, Player.Alpha);
            case "[alpha tiger]":
                return getCardImageStackPane(CardType.Tiger1, Player.Alpha);
            case "[alpha rabbit]":
                return getCardImageStackPane(CardType.Rabbit1, Player.Alpha);
            case "[alpha dragon]":
                return getCardImageStackPane(CardType.Dragon1, Player.Alpha);
            case "[alpha snake]":
                return getCardImageStackPane(CardType.Snake1, Player.Alpha);
            case "[alpha horse]":
                return getCardImageStackPane(CardType.Horse1, Player.Alpha);
            case "[alpha ram]":
                return getCardImageStackPane(CardType.Ram1, Player.Alpha);
            case "[alpha monkey]":
                return getCardImageStackPane(CardType.Monkey1, Player.Alpha);
            case "[alpha rooster]":
                return getCardImageStackPane(CardType.Rooster1, Player.Alpha);
            case "[alpha dog]":
                return getCardImageStackPane(CardType.Dog1, Player.Alpha);
            case "[alpha boar]":
                return getCardImageStackPane(CardType.Boar1, Player.Alpha);

            case "[alpha fish]":
                return getCardImageStackPane(CardType.Fish1, Player.Alpha);
            case "[alpha elephant]":
                return getCardImageStackPane(CardType.Elephant1, Player.Alpha);
            case "[alpha squid]":
                return getCardImageStackPane(CardType.Squid1, Player.Alpha);
            case "[alpha frog]":
                return getCardImageStackPane(CardType.Frog1, Player.Alpha);

            case "[beta rat]":
                return getCardImageStackPane(CardType.Rat1, Player.Beta);
            case "[beta ox]":
                return getCardImageStackPane(CardType.Ox1, Player.Beta);
            case "[beta tiger]":
                return getCardImageStackPane(CardType.Tiger1, Player.Beta);
            case "[beta rabbit]":
                return getCardImageStackPane(CardType.Rabbit1, Player.Beta);
            case "[beta dragon]":
                return getCardImageStackPane(CardType.Dragon1, Player.Beta);
            case "[beta snake]":
                return getCardImageStackPane(CardType.Snake1, Player.Beta);
            case "[beta horse]":
                return getCardImageStackPane(CardType.Horse1, Player.Beta);
            case "[beta ram]":
                return getCardImageStackPane(CardType.Ram1, Player.Beta);
            case "[beta monkey]":
                return getCardImageStackPane(CardType.Monkey1, Player.Beta);
            case "[beta rooster]":
                return getCardImageStackPane(CardType.Rooster1, Player.Beta);
            case "[beta dog]":
                return getCardImageStackPane(CardType.Dog1, Player.Beta);
            case "[beta boar]":
                return getCardImageStackPane(CardType.Boar1, Player.Beta);

            case "[beta fish]":
                return getCardImageStackPane(CardType.Fish1, Player.Beta);
            case "[beta elephant]":
                return getCardImageStackPane(CardType.Elephant1, Player.Beta);
            case "[beta squid]":
                return getCardImageStackPane(CardType.Squid1, Player.Beta);
            case "[beta frog]":
                return getCardImageStackPane(CardType.Frog1, Player.Beta);

            case "[alpha snipe]":
                return getCardImageStackPane(CardType.AlphaSnipe, Player.Alpha);
            case "[beta snipe]":
                return getCardImageStackPane(CardType.BetaSnipe, Player.Beta);

            case "[UNCAPTIONED alpha reserve]":
                return getRowImageStackPane(Row.AlphaReserve);
            case "[rank 1]":
                return getRowImageStackPane(Row.Rank1);
            case "[rank 2]":
                return getRowImageStackPane(Row.Rank2);
            case "[rank 3]":
                return getRowImageStackPane(Row.Rank3);
            case "[rank 4]":
                return getRowImageStackPane(Row.Rank4);
            case "[rank 5]":
                return getRowImageStackPane(Row.Rank5);
            case "[rank 6]":
                return getRowImageStackPane(Row.Rank6);
            case "[UNCAPTIONED beta reserve]":
                return getRowImageStackPane(Row.BetaReserve);

            default:
                throw new IllegalArgumentException("Cannot parse image type: " + bracketedSrc);
        }
    }

    private static StackPane getCardImageStackPane(CardType type, Player allegiance) {
        Card card = new Card(type, allegiance);

        StackPane stackPane = new StackPane();
        stackPane.setPadding(new Insets(6, 3, 6, 3));

        for (Image image : card.getImages()) {
            ImageView view = new ImageView(image);
            view.setPreserveRatio(true);
            view.setFitHeight(CARD_HEIGHT);
            stackPane.getChildren().add(view);
        }
        return stackPane;
    }

    private static StackPane getRowImageStackPane(int row) {
        StackPane rowMarkerContainer = new StackPane();
        rowMarkerContainer.setPadding(new Insets(6, 3, 6, 3));

        ImageView rowMarker = new ImageView(Images.ROW_MARKERS[row]);
        rowMarker.setPreserveRatio(true);
        rowMarker.setFitHeight(CARD_HEIGHT);
        rowMarkerContainer.getChildren().add(rowMarker);

        return rowMarkerContainer;
    }

    private static String getRightCaption(String bracketedSrc) {
        switch (bracketedSrc) {
            case "[alpha rat]":
                return ")";
            case "[alpha ox]":
                return ")";
            case "[alpha tiger]":
                return ")";
            case "[alpha rabbit]":
                return ")";
            case "[alpha dragon]":
                return ")";
            case "[alpha snake]":
                return ")";
            case "[alpha horse]":
                return ")";
            case "[alpha ram]":
                return ")";
            case "[alpha monkey]":
                return ")";
            case "[alpha rooster]":
                return ")";
            case "[alpha dog]":
                return ")";
            case "[alpha boar]":
                return ")";

            case "[alpha fish]":
                return ")";
            case "[alpha elephant]":
                return ")";
            case "[alpha squid]":
                return ")";
            case "[alpha frog]":
                return ")";

            case "[beta rat]":
                return ")";
            case "[beta ox]":
                return ")";
            case "[beta tiger]":
                return ")";
            case "[beta rabbit]":
                return ")";
            case "[beta dragon]":
                return ")";
            case "[beta snake]":
                return ")";
            case "[beta horse]":
                return ")";
            case "[beta ram]":
                return ")";
            case "[beta monkey]":
                return ")";
            case "[beta rooster]":
                return ")";
            case "[beta dog]":
                return ")";
            case "[beta boar]":
                return ")";

            case "[beta fish]":
                return ")";
            case "[beta elephant]":
                return ")";
            case "[beta squid]":
                return ")";
            case "[beta frog]":
                return ")";

            case "[alpha snipe]":
                return ")";
            case "[beta snipe]":
                return ")";

            case "[UNCAPTIONED alpha reserve]":
                return "";
            case "[rank 1]":
                return ")";
            case "[rank 2]":
                return ")";
            case "[rank 3]":
                return ")";
            case "[rank 4]":
                return ")";
            case "[rank 5]":
                return ")";
            case "[rank 6]":
                return ")";
            case "[UNCAPTIONED beta reserve]":
                return "";

            default:
                throw new IllegalArgumentException("Cannot parse image type: " + bracketedSrc);
        }
    }

    @Override
    public ArrayList<Node> render() {
        Text leftCaptionNode = new Text(leftCaption);
        leftCaptionNode.setFont(Font.font(Chunk.FONT_FAMILY, Chunk.FONT_SIZE));

        Text rightCaptionNode = new Text(rightCaption);
        rightCaptionNode.setFont(Font.font(Chunk.FONT_FAMILY, Chunk.FONT_SIZE));

        HBox container = new HBox();
        container.getChildren().addAll(leftCaptionNode, imageStackPane, rightCaptionNode);
        container.setAlignment(Pos.CENTER_LEFT);

        ArrayList<Node> nodes = new ArrayList<>();
        nodes.add(container);
        return nodes;
    }
}

class AwaitActionParagraph implements Paragraph {
    public static AwaitActionParagraph legal(Action action) {
        return new AwaitActionParagraph(action, true);
    }

    public static AwaitActionParagraph illegal(Action action) {
        return new AwaitActionParagraph(action, false);
    }

    private Action action;
    private boolean legal;

    private AwaitActionParagraph(Action action, boolean legal) {
        this.action = action;
        this.legal = legal;
    }

    public Action getExpectedAction() {
        return action;
    }

    public boolean isLegal() {
        return legal;
    }

    @Override
    public ArrayList<Node> render() {
        return new ArrayList<>();
    }
}

class AwaitClickParagraph implements Paragraph {
    private Runnable onNextButtonClicked;

    public AwaitClickParagraph(Runnable onNextButtonClicked) {
        this.onNextButtonClicked = onNextButtonClicked;
    }

    @Override
    public ArrayList<Node> render() {
        Button nextButton = new Button("Next");
        nextButton.setPadding(new Insets(6, 10, 6, 10));
        nextButton.setOnMouseClicked((_e) -> {
            onNextButtonClicked.run();
        });

        ArrayList<Node> nodes = new ArrayList<>();
        nodes.add(nextButton);
        return nodes;
    }
}

class PerformActionParagraph implements Paragraph {
    private Action action;

    public PerformActionParagraph(Action action) {
        this.action = action;
    }

    public Action getPerformedAction() {
        return action;
    }

    @Override
    public ArrayList<Node> render() {
        return new ArrayList<>();
    }
}

class EndTutorialParagraph implements Paragraph {
    public EndTutorialParagraph() {

    }

    @Override
    public ArrayList<Node> render() {
        return new ArrayList<>();
    }
}

class ParagraphParser {
    public static Paragraph parse(String untrimmed, Runnable onNextButtonClicked) {
        final String src = untrimmed.trim();

        if (src.startsWith("[await step]") || src.startsWith("[await illegal step]")) {
            int movedCardStart = StringUtil.nthIndexOf(src, "[", 2);
            int movedCardEnd = StringUtil.nthIndexOf(src, "]", 2) + 1;
            int destinationStart = StringUtil.nthIndexOf(src, "[", 3);
            int destinationEnd = StringUtil.nthIndexOf(src, "]", 3) + 1;

            CardType movedCard = parseCardType(src.substring(movedCardStart, movedCardEnd));
            int destination = parseRank(src.substring(destinationStart, destinationEnd));

            final Action action;
            if (movedCard.isSnipe()) {
                action = Action.snipeStep(destination);
            } else {
                action = Action.animalStep(movedCard, destination);
            }

            if (src.startsWith("[await illegal step]")) {
                return AwaitActionParagraph.illegal(action);
            } else {
                return AwaitActionParagraph.legal(action);
            }
        }

        if (src.startsWith("[await drop]") || src.startsWith("[await illegal drop]")) {
            int droppedStart = StringUtil.nthIndexOf(src, "[", 2);
            int droppedEnd = StringUtil.nthIndexOf(src, "]", 2) + 1;
            int destinationStart = StringUtil.nthIndexOf(src, "[", 3);
            int destinationEnd = StringUtil.nthIndexOf(src, "]", 3) + 1;

            CardType dropped = parseCardType(src.substring(droppedStart, droppedEnd));
            int destination = parseRank(src.substring(destinationStart, destinationEnd));

            Action action = Action.drop(dropped, destination);

            if (src.startsWith("[await illegal drop]")) {
                return AwaitActionParagraph.illegal(action);
            } else {
                return AwaitActionParagraph.legal(action);
            }
        }

        if (src.startsWith("[await click]")) {
            return new AwaitClickParagraph(onNextButtonClicked);
        }

        if (src.startsWith("[perform computer step]")) {
            int movedCardStart = StringUtil.nthIndexOf(src, "[", 2);
            int movedCardEnd = StringUtil.nthIndexOf(src, "]", 2) + 1;
            int destinationStart = StringUtil.nthIndexOf(src, "[", 3);
            int destinationEnd = StringUtil.nthIndexOf(src, "]", 3) + 1;

            CardType movedCard = parseCardType(src.substring(movedCardStart, movedCardEnd));
            int destination = parseRank(src.substring(destinationStart, destinationEnd));

            final Action action;
            if (movedCard.isSnipe()) {
                action = Action.snipeStep(destination);
            } else {
                action = Action.animalStep(movedCard, destination);
            }

            return new PerformActionParagraph(action);
        }

        if (src.startsWith("[perform computer drop]")) {
            int droppedStart = StringUtil.nthIndexOf(src, "[", 2);
            int droppedEnd = StringUtil.nthIndexOf(src, "]", 2) + 1;
            int destinationStart = StringUtil.nthIndexOf(src, "[", 3);
            int destinationEnd = StringUtil.nthIndexOf(src, "]", 3) + 1;

            CardType dropped = parseCardType(src.substring(droppedStart, droppedEnd));
            int destination = parseRank(src.substring(destinationStart, destinationEnd));

            Action action = Action.drop(dropped, destination);
            return new PerformActionParagraph(action);
        }

        if (src.startsWith("[end tutorial]")) {
            return new EndTutorialParagraph();
        }

        return VisibleParagraph.parse(src);

    }

    private static CardType parseCardType(String s) {
        switch (s) {
            case "[rat 1]":
                return CardType.Rat1;
            case "[ox 1]":
                return CardType.Ox1;
            case "[tiger 1]":
                return CardType.Tiger1;
            case "[rabbit 1]":
                return CardType.Rabbit1;
            case "[dragon 1]":
                return CardType.Dragon1;
            case "[snake 1]":
                return CardType.Snake1;
            case "[horse 1]":
                return CardType.Horse1;
            case "[ram 1]":
                return CardType.Ram1;
            case "[monkey 1]":
                return CardType.Monkey1;
            case "[rooster 1]":
                return CardType.Rooster1;
            case "[dog 1]":
                return CardType.Dog1;
            case "[boar 1]":
                return CardType.Boar1;

            case "[fish 1]":
                return CardType.Fish1;
            case "[elephant 1]":
                return CardType.Elephant1;
            case "[squid 1]":
                return CardType.Squid1;
            case "[frog 1]":
                return CardType.Frog1;

            case "[rat 2]":
                return CardType.Rat2;
            case "[ox 2]":
                return CardType.Ox2;
            case "[tiger 2]":
                return CardType.Tiger2;
            case "[rabbit 2]":
                return CardType.Rabbit2;
            case "[dragon 2]":
                return CardType.Dragon2;
            case "[snake 2]":
                return CardType.Snake2;
            case "[horse 2]":
                return CardType.Horse2;
            case "[ram 2]":
                return CardType.Ram2;
            case "[monkey 2]":
                return CardType.Monkey2;
            case "[rooster 2]":
                return CardType.Rooster2;
            case "[dog 2]":
                return CardType.Dog2;
            case "[boar 2]":
                return CardType.Boar2;

            case "[fish 2]":
                return CardType.Fish2;
            case "[elephant 2]":
                return CardType.Elephant2;
            case "[squid 2]":
                return CardType.Squid2;
            case "[frog 2]":
                return CardType.Frog2;

            case "[alpha snipe]":
                return CardType.AlphaSnipe;
            case "[beta snipe]":
                return CardType.BetaSnipe;

            default:
                throw new IllegalArgumentException("Cannot parse card type: " + s);
        }
    }

    private static int parseRank(String s) {
        switch (s) {
            case "[rank 1]":
                return 1;

            case "[rank 2]":
                return 2;

            case "[rank 3]":
                return 3;

            case "[rank 4]":
                return 4;

            case "[rank 5]":
                return 5;

            case "[rank 6]":
                return 6;

            default:
                throw new IllegalArgumentException("Cannot parse rank: " + s);
        }
    }
}

class StringUtil {
    /**
     * Returns the index of the nth occurence of specified substring, or -1 if there
     * are less than n occurences in the source string.
     * 
     * N is one-based. If n is less than 1, this method throws an exception.
     * 
     * Example:
     * 
     * <ul>
     * 
     * <li><code>nthIndexOf("foo", "o", 1)</code> -> <code>1</code></li>
     * 
     * <li><code>nthIndexOf("foo", "o", 2)</code> -> <code>2</code></li>
     * 
     * <li><code>nthIndexOf("foo", "o", 3)</code> -> <code>-1</code></li>
     * 
     * </ul>
     * 
     * 
     * If there are overlapping matches, no matches will be counted until the end of
     * the first match. Example:
     * 
     * <ul>
     * 
     * <li><code>nthIndexOf("oooo", "oo", 1)</code> -> <code>0</code></li>
     * 
     * <li><code>nthIndexOf("oooo", "oo", 2)</code> -> <code>2</code></li>
     * 
     * <li><code>nthIndexOf("oooo", "oo", 3)</code> -> <code>-1</code></li>
     * 
     * </ul>
     * 
     * @param s      The string to search in.
     * @param substr The substring to search for.
     * @param n      The one-based number of the occurence you want to find the
     *               index of.
     * @return
     */
    public static int nthIndexOf(String s, String substr, int n) {
        if (n < 1) {
            throw new IllegalArgumentException("N cannot be less than 1, but you passed in " + n);
        }

        int i = 0;
        int occurence = 0;

        while (true) {
            i = s.indexOf(substr, i);

            if (i == -1) {
                return -1;
            }

            occurence++;
            if (occurence == n) {
                return i;
            }

            i += substr.length();
        }
    }

    public static int leastIndexOf(String src, int start, String... substrs) {
        int i = -1;
        for (String substr : substrs) {
            int index = src.indexOf(substr, start);
            if (index != -1 && (index < i || i == -1)) {
                i = index;
            }
        }
        return i;
    }

    /**
     * <code>getWordsAndSpaces("foo bar\nbaz")</code> ->
     * <code>["foo", " ", "bar", "\n", "baz"]</code>
     */
    public static ArrayList<String> getWordsAndSpaces(String src) {
        ArrayList<String> words = new ArrayList<>();

        int i = 0;
        int j = StringUtil.leastIndexOf(src, i, " ", "\n");
        while (j != -1) {
            words.add(src.substring(i, j));
            words.add(src.substring(j, j + 1));
            i = j + 1;
            j = StringUtil.leastIndexOf(src, i, " ", "\n");
        }

        if (i < src.length()) {
            words.add(src.substring(i));
        }

        return words;
    }

}