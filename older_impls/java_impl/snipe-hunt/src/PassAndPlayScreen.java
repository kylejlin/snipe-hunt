import java.util.ArrayList;
import java.util.List;

import javafx.application.Platform;
import javafx.beans.Observable;
import javafx.event.ActionEvent;
import javafx.event.EventHandler;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.layout.HBox;
import javafx.scene.Node;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.scene.control.Alert;
import javafx.scene.control.Button;
import javafx.scene.control.ButtonType;
import javafx.scene.control.ScrollPane;
import javafx.scene.control.Alert.AlertType;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.layout.Background;
import javafx.scene.layout.BackgroundFill;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;
import javafx.scene.paint.Color;
import javafx.scene.shape.Rectangle;
import javafx.scene.text.Font;
import javafx.scene.text.Text;
import javafx.stage.Stage;

public class PassAndPlayScreen {
    private static final double CARD_HEIGHT = 75;
    private static final double HISTORY_PANE_WIDTH = 800;
    private static final double MAX_INSTRUCTIONS_BOX_HEIGHT = 696;

    public static final double INSTRUCTIONS_BOX_WIDTH = 1200 - 780;

    private Stage stage;
    private HBox rowsAndInstructionsContainer;
    private VBox rowsContainer;
    private HBox[] rowViews;
    private ScrollPane instructionsScrollPane;
    private VBox instructionsContainer;
    private HBox historyTextContainer;

    private GameState state;
    private ActionRestricter restricter;
    private CardType selected;

    private ArrayList<Runnable> abortListeners;

    public PassAndPlayScreen(Stage stage, GameState state) {
        this(stage, state, ActionRestricter.noRestrictions());
    }

    public PassAndPlayScreen(Stage stage, GameState state, ActionRestricter restricter) {
        this.stage = stage;
        instructionsContainer = null;
        instructionsScrollPane = null;

        this.state = state;
        this.restricter = restricter;
        selected = null;

        abortListeners = new ArrayList<>();
    }

    public void init() {
        BorderPane root = new BorderPane();
        Scene scene = new Scene(root, 1200, 900);
        stage.setScene(scene);

        VBox table = new VBox();
        root.setCenter(table);

        rowsAndInstructionsContainer = new HBox();
        table.getChildren().add(rowsAndInstructionsContainer);

        rowsContainer = new VBox();
        rowsAndInstructionsContainer.getChildren().add(rowsContainer);

        HBox alphaReserve = new HBox();
        HBox row1 = new HBox();
        HBox row2 = new HBox();
        HBox row3 = new HBox();
        HBox row4 = new HBox();
        HBox row5 = new HBox();
        HBox row6 = new HBox();
        HBox betaReserve = new HBox();
        rowsContainer.getChildren().addAll(alphaReserve, row1, row2, row3, row4, row5, row6, betaReserve);

        rowViews = new HBox[] { alphaReserve, row1, row2, row3, row4, row5, row6, betaReserve };

        initLocationColors();
        addLocationClickListeners();

        HBox historyView = new HBox();
        table.getChildren().add(historyView);
        historyView.setAlignment(Pos.CENTER_RIGHT);
        historyView.setSpacing(20);
        historyView.setMaxWidth(1200);

        Button abortButton = new Button("Abort Game");
        historyView.getChildren().add(abortButton);
        abortButton.setPadding(new Insets(6, 10, 6, 10));
        addAbortButtonListener(abortButton);

        Button undoButton = new Button("Undo");
        historyView.getChildren().add(undoButton);
        undoButton.setPadding(new Insets(6, 10, 6, 10));

        addUndoButtonListener(undoButton);

        ScrollPane scrollPane = new ScrollPane();
        historyView.getChildren().add(scrollPane);
        scrollPane.setPrefHeight(scene.getHeight());
        scrollPane.setPrefWidth(HISTORY_PANE_WIDTH);

        historyTextContainer = new HBox();
        scrollPane.setContent(historyTextContainer);
        historyTextContainer.setAlignment(Pos.CENTER_LEFT);
        historyTextContainer.setPrefHeight(75);

        initAutoScroll(historyTextContainer.widthProperty(), scrollPane);

        render();

    }

    private void initLocationColors() {
        rowViews[Row.AlphaReserve].setBackground(getBackgroundFromHex(0x222222));
        rowViews[Row.Rank1].setBackground(getBackgroundFromHex(0x6f89b0));
        rowViews[Row.Rank2].setBackground(getBackgroundFromHex(0x285fb2));
        rowViews[Row.Rank3].setBackground(getBackgroundFromHex(0x6f89b0));

        rowViews[Row.Rank4].setBackground(getBackgroundFromHex(0xe69685));
        rowViews[Row.Rank5].setBackground(getBackgroundFromHex(0xb71d06));
        rowViews[Row.Rank6].setBackground(getBackgroundFromHex(0xe69685));
        rowViews[Row.BetaReserve].setBackground(getBackgroundFromHex(0x222222));
    }

    private static Background getBackgroundFromHex(int hex) {
        return new Background(
                new BackgroundFill(Color.rgb((hex >>> 16) & 0xFF, (hex >>> 8) & 0xFF, hex & 0xFF), null, null));
    }

    private void addLocationClickListeners() {
        for (final int rowNumber : Row.RANKS) {
            rowViews[rowNumber].setOnMouseClicked(_e -> {
                handleRowClick(rowNumber);
            });
        }
    }

    private void handleRowClick(int rowNumber) {
        if (state.isGameOver()) {
            return;
        }

        if (!restricter.canUserPlayFor(state.turn)) {
            return;
        }

        if (selected != null) {
            Action action;
            if (selected.isSnipe()) {
                action = Action.snipeStep(rowNumber);
            } else if (state.isInReserve(selected)) {
                action = Action.drop(selected, rowNumber);
            } else {
                action = Action.animalStep(selected, rowNumber);
            }

            selected = null;

            this.performActionOrAlertError(action);
        }
    }

    public void performActionOrAlertError(Action action) {
        if (!restricter.isAuthorized(action)) {
            restricter.onUnauthorizedActionAttempt(action);
            selected = null;
            render();
            return;
        }

        Object newState = state.tryPerform(action);
        if (newState instanceof GameState) {
            state = (GameState) newState;
            restricter.onSuccessfulAction(action);
        } else {
            alertUserOfIllegalAction((IllegalGameStateUpdate) newState);
            restricter.onIllegalActionAttempt(action);
        }

        render();
    }

    private static void alertUserOfIllegalAction(IllegalGameStateUpdate reason) {
        Alert a = new Alert(AlertType.ERROR, "" + reason, ButtonType.OK);
        a.setHeaderText("Illegal action:");
        a.showAndWait();
    }

    private static void initAutoScroll(Observable observable, ScrollPane historyScrollPane) {
        observable.addListener((_observable) -> {
            historyScrollPane.setHvalue(1.0);
        });
    }

    private void addAbortButtonListener(Button button) {
        button.setOnMouseClicked(_event -> {
            askForAbortionConfirmation();
        });
    }

    private void askForAbortionConfirmation() {
        Alert a = new Alert(AlertType.WARNING,
                "Are you sure you want to leave this game?\n\nIf you leave this game, all the game data will be permanently deleted.",
                ButtonType.OK, ButtonType.CANCEL);
        a.setHeaderText("Confirm exit");

        Button okButton = (Button) a.getDialogPane().lookupButton(ButtonType.OK);
        ClickTracker tracker = new ClickTracker();
        okButton.addEventFilter(ActionEvent.ACTION, tracker);

        a.showAndWait();

        if (tracker.wasButtonClicked()) {
            abortGame();
        }
    }

    private void abortGame() {
        for (Runnable listener : abortListeners) {
            listener.run();
        }
    }

    private void addUndoButtonListener(Button button) {
        button.setOnMouseClicked(_event -> {
            if (state.actions.isEmpty()) {
                alertUserOfIllegalUndo();
            } else {
                forceUndo();
            }
        });
    }

    private static void alertUserOfIllegalUndo() {
        Alert a = new Alert(AlertType.INFORMATION, "Nothing to undo.", ButtonType.OK);
        a.setHeaderText("Cannot undo");
        a.showAndWait();
    }

    private void forceUndo() {
        forceUndo(false);
    }

    private void forceUndo(boolean suppressNotifications) {
        state = state.forceUndo();
        selected = null;

        if (!suppressNotifications) {
            restricter.onSuccessfulUndo();
        }

        render();
    }

    public void forceUndoWithoutNotifying() {
        forceUndo(true);
    }

    public void render() {
        if (instructionsContainer == null) {
            rowsContainer.setMinWidth(1200);
            rowsContainer.setMaxWidth(1200);
            rowsContainer.setClip(new Rectangle(Double.MAX_VALUE, Double.MAX_VALUE));
        }

        for (int location : Row.ALL) {
            HBox box = rowViews[location];
            box.getChildren().clear();

            StackPane rowMarkerContainer = new StackPane();
            box.getChildren().add(rowMarkerContainer);
            rowMarkerContainer.setPadding(new Insets(6, 3, 6, 3));

            ImageView rowMarker = new ImageView(Images.ROW_MARKERS[location]);
            rowMarker.setPreserveRatio(true);
            rowMarker.setFitHeight(CARD_HEIGHT);
            rowMarkerContainer.getChildren().add(rowMarker);

            List<Card> cards = state.currentBoard[location];
            for (final Card card : cards) {
                StackPane stackPane = new StackPane();
                stackPane.setPadding(new Insets(6, 3, 6, 3));

                for (Image image : card.getImages()) {
                    ImageView view = new ImageView(image);
                    view.setPreserveRatio(true);
                    view.setFitHeight(CARD_HEIGHT);
                    stackPane.getChildren().add(view);
                }

                if (card.type == selected) {
                    ImageView selectionOverlay = new ImageView(Images.SELECTION_OVERLAY);
                    selectionOverlay.setPreserveRatio(true);
                    selectionOverlay.setFitHeight(CARD_HEIGHT);
                    stackPane.getChildren().add(selectionOverlay);
                }

                stackPane.setOnMouseClicked((event) -> {
                    event.consume();

                    if (state.isGameOver()) {
                        return;
                    }

                    if (!restricter.canUserPlayFor(state.turn)) {
                        return;
                    }

                    if (card.allegiance != state.turn) {
                        return;
                    }

                    if (selected == card.type) {
                        selected = null;
                    } else {
                        selected = card.type;
                    }

                    render();
                });

                box.getChildren().add(stackPane);
            }
        }

        renderHistory();
    }

    private void renderHistory() {
        historyTextContainer.getChildren().clear();
        historyTextContainer.getChildren().addAll(getHistoryTextNodes());
    }

    private ArrayList<Text> getHistoryTextNodes() {
        final Color[] COLORS = { colorFromHex(0x285fb2), colorFromHex(0xb71d06) };

        ArrayList<Text> nodes = new ArrayList<>();

        {
            Text turn1 = new Text("1" + Player.Beta.getGreekLetter() + ". <deal cards>; ");
            turn1.setFont(new Font(30));
            turn1.setFill(COLORS[Player.Beta.raw]);
            nodes.add(turn1);

            Text turn2 = new Text("2" + Player.Alpha.getGreekLetter() + ". <deal cards>; ");
            turn2.setFont(new Font(30));
            turn2.setFill(COLORS[Player.Alpha.raw]);
            nodes.add(turn2);
        }

        int turnNumber = 3;
        boolean wasLastActionInitialAnimalStep = false;

        for (Action action : state.actions) {
            Player turn = turnNumber % 2 == 0 ? Player.Alpha : Player.Beta;
            String emoji = turn.getGreekLetter();
            if (action.actionType == Action.SnipeStep) {
                Text newText = new Text(turnNumber + emoji + ". " + emoji + " -> " + action.destination + "; ");
                newText.setFont(new Font(30));
                newText.setFill(COLORS[turn.raw]);
                nodes.add(newText);
                turnNumber++;
            } else if (action.actionType == Action.Drop) {
                Text newText = new Text(turnNumber + emoji + ". !" + action.dropped.toUnnumberedString() + " -> "
                        + action.destination + "; ");
                newText.setFont(new Font(30));
                newText.setFill(COLORS[turn.raw]);
                nodes.add(newText);
                turnNumber++;
            } else {
                if (wasLastActionInitialAnimalStep) {
                    Text oldText = nodes.get(nodes.size() - 1);
                    oldText.setText(oldText.getText() + ", " + action.moved.toUnnumberedString() + " -> "
                            + action.destination + "; ");
                    wasLastActionInitialAnimalStep = false;
                    turnNumber++;
                } else {
                    Text newText = new Text(turnNumber + emoji + ". " + action.moved.toUnnumberedString() + " -> "
                            + action.destination);
                    newText.setFont(new Font(30));
                    newText.setFill(COLORS[turn.raw]);
                    nodes.add(newText);
                    wasLastActionInitialAnimalStep = true;
                }
            }
        }

        {
            Text last = nodes.get(nodes.size() - 1);
            String s = last.getText();

            if (s.endsWith("; ")) {
                last.setText(s.substring(0, s.length() - 1));
            } else if (wasLastActionInitialAnimalStep) {
                Text newText = new Text("...");
                newText.setFont(new Font(30));
                nodes.add(newText);
            }
        }

        {
            Player winner = state.getWinner();
            if (winner == null) {
                boolean isAtBeginningOfTurn = !wasLastActionInitialAnimalStep;
                if (isAtBeginningOfTurn) {
                    Text turnNumberText = new Text(" " + turnNumber + state.turn.getGreekLetter() + ".");
                    turnNumberText.setFont(new Font(30));
                    turnNumberText.setFill(COLORS[state.turn.raw]);
                    nodes.add(turnNumberText);
                }

                Text activePlayerText = new Text(" (" + state.turn + " to play)");
                activePlayerText.setFont(new Font(30));
                nodes.add(activePlayerText);
            } else {
                Text winnerText = new Text(" (" + winner + " is victorious)");
                winnerText.setFont(new Font(30));
                nodes.add(winnerText);
            }
        }

        return nodes;
    }

    private Color colorFromHex(int hex) {
        return Color.rgb((hex >>> 16) & 0xFF, (hex >>> 8) & 0xFF, hex & 0xFF);
    }

    public void onGameAborted(Runnable listener) {
        abortListeners.add(listener);
    }

    public GameState getState() {
        return state;
    }

    public void printInstructions(List<? extends Node> instructions) {
        if (instructionsContainer == null) {
            initInstructionsContainer();
        }

        instructionsContainer.getChildren().clear();
        instructionsContainer.getChildren().addAll(instructions);

        rowsContainer.setMinWidth(1200 - INSTRUCTIONS_BOX_WIDTH);
        rowsContainer.setMaxWidth(1200 - INSTRUCTIONS_BOX_WIDTH);
        rowsContainer.setClip(new Rectangle(1200 - INSTRUCTIONS_BOX_WIDTH, Double.MAX_VALUE));
    }

    private void initInstructionsContainer() {
        VBox instructionsContainer = new VBox();
        this.instructionsContainer = instructionsContainer;
        rowsAndInstructionsContainer.getChildren().add(instructionsContainer);

        instructionsContainer.heightProperty().addListener(_obs -> {
            double boxHeight = instructionsContainer.getHeight();
            Parent parent = instructionsContainer.getParent();

            if (boxHeight > MAX_INSTRUCTIONS_BOX_HEIGHT && parent == rowsAndInstructionsContainer) {
                instructionsScrollPane = new ScrollPane();

                int index = rowsAndInstructionsContainer.getChildren().indexOf(instructionsContainer);
                rowsAndInstructionsContainer.getChildren().remove(index);

                instructionsScrollPane.setContent(instructionsContainer);
                instructionsScrollPane.setMinHeight(MAX_INSTRUCTIONS_BOX_HEIGHT);
                instructionsScrollPane.setMaxHeight(MAX_INSTRUCTIONS_BOX_HEIGHT);

                rowsAndInstructionsContainer.getChildren().add(index, instructionsScrollPane);

                Platform.runLater(() -> {
                    render();
                });

                return;
            }

            if (boxHeight <= MAX_INSTRUCTIONS_BOX_HEIGHT && parent != rowsAndInstructionsContainer) {
                int index = rowsAndInstructionsContainer.getChildren().indexOf(instructionsScrollPane);
                rowsAndInstructionsContainer.getChildren().remove(index);
                rowsAndInstructionsContainer.getChildren().add(index, instructionsContainer);

                Platform.runLater(() -> {
                    render();
                });

                return;
            }

        });

    }

    public void hideInstructions() {
        if (instructionsContainer != null) {
            deinitInstructionsContainer();
        }

        rowsContainer.setMinWidth(1200);
        rowsContainer.setMaxWidth(1200);
        rowsContainer.setClip(new Rectangle(Double.MAX_VALUE, Double.MAX_VALUE));
    }

    private void deinitInstructionsContainer() {
        rowsAndInstructionsContainer.getChildren().remove(instructionsContainer);
        instructionsContainer = null;
    }

}

class ClickTracker implements EventHandler<ActionEvent> {
    private boolean clicked = false;

    @Override
    public void handle(ActionEvent event) {
        clicked = true;
    }

    public boolean wasButtonClicked() {
        return clicked;
    }
}