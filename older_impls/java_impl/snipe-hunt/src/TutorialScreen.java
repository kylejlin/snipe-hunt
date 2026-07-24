
import java.io.File;
import java.io.FileNotFoundException;
import java.util.ArrayList;
import java.util.Scanner;

import javafx.scene.control.Alert;
import javafx.scene.control.ButtonType;
import javafx.scene.control.Alert.AlertType;
import javafx.stage.Stage;

public class TutorialScreen {
    private static GameState getInitialState() {
        return GameState.fromInitialBoardSpec(new CardType[][] {

                { CardType.Rabbit1 },

                { CardType.Ram1, CardType.AlphaSnipe, CardType.Snake1 },

                { CardType.Rabbit2, CardType.Rooster1, CardType.Elephant1, CardType.Boar1, CardType.Horse1,
                        CardType.Monkey1, CardType.Rat1, CardType.Ox1, CardType.Dragon1, CardType.Fish1, CardType.Fish2,
                        CardType.Dog1 },

                { CardType.Rooster2 },

                { CardType.Squid1 },

                { CardType.Elephant2, CardType.Dragon2, CardType.Ram2, CardType.Rat2, CardType.Ox2, CardType.Dog2,
                        CardType.Monkey2, CardType.Squid2, CardType.Frog1, CardType.Frog2, CardType.Horse2,
                        CardType.Tiger1 },

                { CardType.Snake2, CardType.BetaSnipe, CardType.Tiger2 },

                { CardType.Boar2 },

        });
    }

    private GameState state;
    private TutorialActionRestricter restricter;
    private PassAndPlayScreen passAndPlayScreen;

    private ArrayList<Page> pages;
    private int currentPageIndex;

    // o
    public TutorialScreen(Stage stage) {
        state = getInitialState();
        restricter = new TutorialActionRestricter();
        passAndPlayScreen = new PassAndPlayScreen(stage, state, restricter);

        pages = getTutorialPages();
        currentPageIndex = 0;
    }

    private ArrayList<Page> getTutorialPages() {
        String tutorialPath = TutorialScreen.class.getClassLoader().getResource("resources/tutorial.md").getFile();
        try {
            Scanner s = new Scanner(new File(tutorialPath));
            s.useDelimiter("\\Z");
            String tutorialSrc = s.next();
            return Page.parse(tutorialSrc, () -> {
                this.onNextButtonClicked();
            });
        } catch (FileNotFoundException _e) {
            throw new IllegalStateException("Cannot find tutorial.md");
        }
    }

    private void onNextButtonClicked() {
        currentPageIndex++;
        performComputerActionIfNeeded();
        render();
    }

    private void performComputerActionIfNeeded() {
        Page currentPage = expectCurrentPage();
        Action performed = currentPage.getPerformedAction();

        if (performed != null) {
            passAndPlayScreen.performActionOrAlertError(performed);
        }
    }

    public void init() {
        passAndPlayScreen.init();
        passAndPlayScreen.printInstructions(pages.get(currentPageIndex).render());
    }

    private class TutorialActionRestricter implements ActionRestricter {
        @Override
        public boolean isAuthorized(Action action) {
            Page currentPage = expectCurrentPage();
            return action.equals(currentPage.getExpectedAction()) || action.equals(currentPage.getPerformedAction());
        }

        @Override
        public void onSuccessfulAction(Action action) {
            Page currentPage = expectCurrentPage();

            if (action.equals(currentPage.getExpectedAction())) {
                currentPageIndex++;
                performComputerActionIfNeeded();
            }

            render();
        }

        @Override
        public void onIllegalActionAttempt(Action action) {
            Action expectedAction = expectCurrentPage().getExpectedAction();
            if (action.equals(expectedAction)) {
                currentPageIndex++;
                render();
            }
        }

        @Override
        public void onUnauthorizedActionAttempt(Action action) {
            alertUnauthorizedAction(action);
        }

        @Override
        public void onSuccessfulUndo() {
            updateCurrentPageIndexAfterUndo();
            render();
        }

        @Override
        public boolean canUserPlayFor(Player player) {
            Page currentPage = tryGetCurrentPage();
            return currentPage != null && currentPage.getExpectedAction() != null && player == Player.Beta;
        }
    }

    private Page expectCurrentPage() {
        if (currentPageIndex < pages.size()) {
            return pages.get(currentPageIndex);
        }

        throw new IllegalStateException(
                "expectCurrentPage() was called when the user had already completed the tutorial.");
    }

    private Page tryGetCurrentPage() {
        if (currentPageIndex < pages.size()) {
            return pages.get(currentPageIndex);
        }

        return null;
    }

    private void updateCurrentPageIndexAfterUndo() {
        Page initialPage = expectCurrentPage();

        if (initialPage.getPerformedAction() != null) {
            currentPageIndex--;
            if (expectCurrentPage().getExpectedLegalAction() != null) {
                passAndPlayScreen.forceUndoWithoutNotifying();
            }

            return;
        }

        if (initialPage.getExpectedLegalAction() != null) {
            while (true) {
                currentPageIndex--;
                if (expectCurrentPage().getExpectedLegalAction() != null) {
                    return;
                }
                if (expectCurrentPage().getPerformedAction() != null) {
                    currentPageIndex--;
                    if (expectCurrentPage().getExpectedLegalAction() != null) {
                        passAndPlayScreen.forceUndoWithoutNotifying();
                    }
                    return;
                }
            }
        }

        while (true) {
            currentPageIndex--;
            if (expectCurrentPage().getExpectedLegalAction() != null) {
                return;
            }
            if (expectCurrentPage().getPerformedAction() != null) {
                currentPageIndex--;
                if (expectCurrentPage().getExpectedLegalAction() != null) {
                    passAndPlayScreen.forceUndoWithoutNotifying();
                }
                return;
            }
        }
    }

    private void render() {
        passAndPlayScreen.printInstructions(expectCurrentPage().render());
    }

    private void alertUnauthorizedAction(Action action) {
        Action expectedAction = expectCurrentPage().getExpectedAction();

        Alert alert = new Alert(AlertType.INFORMATION,
                "Please follow the tutorial and " + getActionVerbPhrase(expectedAction) + ".", ButtonType.OK);
        alert.setHeaderText("Please follow the tutorial");
        alert.showAndWait();
    }

    private static String getActionVerbPhrase(Action action) {
        if (action.actionType == Action.SnipeStep) {
            return "move your snipe to rank " + action.destination;
        } else if (action.actionType == Action.Drop) {
            return "drop your " + action.dropped.toUnnumberedString().toLowerCase() + " on rank " + action.destination;
        } else {
            return "move your " + action.moved.toUnnumberedString().toLowerCase() + " to rank " + action.destination;
        }
    }

    public void onGameAborted(Runnable listener) {
        passAndPlayScreen.onGameAborted(listener);
    }
}