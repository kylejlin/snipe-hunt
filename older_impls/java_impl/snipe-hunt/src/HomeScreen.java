import java.util.ArrayList;

import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.VBox;
import javafx.scene.text.Font;
import javafx.scene.text.Text;
import javafx.stage.Stage;

public class HomeScreen {
    private Stage stage;
    private ArrayList<Runnable> passAndPlayListeners;
    private ArrayList<Runnable> learnButtonListeners;

    public HomeScreen(Stage stage) {
        this.stage = stage;
        passAndPlayListeners = new ArrayList<>();
        learnButtonListeners = new ArrayList<>();
    }

    public void init() {
        BorderPane root = new BorderPane();
        Scene scene = new Scene(root, 1200, 900);
        stage.setScene(scene);

        Text titleText = new Text("Snipe Hunt");
        root.setCenter(titleText);
        titleText.setFont(new Font(30));

        VBox buttonsBox = new VBox();
        root.setBottom(buttonsBox);
        buttonsBox.setAlignment(Pos.CENTER);
        buttonsBox.setPadding(new Insets(20, 20, 20, 20));
        buttonsBox.setSpacing(20);

        Button passAndPlayButton = new Button("Start 2-Player Game");
        buttonsBox.getChildren().add(passAndPlayButton);

        Button learnButton = new Button("Learn");
        buttonsBox.getChildren().add(learnButton);

        passAndPlayButton.setOnMouseClicked(_event -> {
            for (Runnable listener : passAndPlayListeners) {
                listener.run();
            }
        });

        learnButton.setOnMouseClicked(_event -> {
            for (Runnable listener : learnButtonListeners) {
                listener.run();
            }
        });
    }

    public void onPassAndPlayButtonClicked(Runnable listener) {
        passAndPlayListeners.add(listener);
    }

    public void onLearnButtonClicked(Runnable listener) {
        learnButtonListeners.add(listener);
    }
}