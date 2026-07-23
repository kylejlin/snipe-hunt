import java.util.ArrayList;

import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.VBox;
import javafx.stage.Stage;

public class LearnScreen {
    private Stage stage;
    private ArrayList<Runnable> backButtonListeners;
    private ArrayList<Runnable> tutorialButtonListeners;
    private ArrayList<Runnable> guideButtonListeners;

    public LearnScreen(Stage stage) {
        this.stage = stage;

        backButtonListeners = new ArrayList<>();
        tutorialButtonListeners = new ArrayList<>();
        guideButtonListeners = new ArrayList<>();
    }

    public void init() {
        BorderPane root = new BorderPane();
        Scene scene = new Scene(root, 1200, 900);
        stage.setScene(scene);

        VBox top = new VBox();
        root.setTop(top);

        Button backButton = new Button("Back");
        top.getChildren().add(backButton);

        VBox center = new VBox();
        root.setCenter(center);
        center.setAlignment(Pos.CENTER);
        center.setPadding(new Insets(20, 20, 20, 20));
        center.setSpacing(20);

        Button tutorialButton = new Button("Start Interactive Tutorial");
        center.getChildren().add(tutorialButton);

        Button readTheGuideButton = new Button("Read the Guide");
        center.getChildren().add(readTheGuideButton);

        backButton.setOnMouseClicked(_event -> {
            for (Runnable listener : backButtonListeners) {
                listener.run();
            }
        });

        tutorialButton.setOnMouseClicked(_event -> {
            for (Runnable listener : tutorialButtonListeners) {
                listener.run();
            }
        });

        readTheGuideButton.setOnMouseClicked(_event -> {
            for (Runnable listener : guideButtonListeners) {
                listener.run();
            }
        });
    }

    public void onBackButtonClicked(Runnable listener) {
        backButtonListeners.add(listener);
    }

    public void onTutorialButtonClicked(Runnable listener) {
        tutorialButtonListeners.add(listener);
    }

    public void onGuideButtonClicked(Runnable listener) {
        guideButtonListeners.add(listener);
    }
}