import java.util.ArrayList;

import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.HBox;
import javafx.scene.web.WebView;
import javafx.stage.Stage;

public class GuideScreen {
    private Stage stage;
    private ArrayList<Runnable> backButtonListeners;

    public GuideScreen(Stage stage) {
        this.stage = stage;
        backButtonListeners = new ArrayList<>();
    }

    public void init() {
        BorderPane root = new BorderPane();
        Scene scene = new Scene(root, 1200, 900);
        stage.setScene(scene);

        HBox headerBox = new HBox();
        root.setTop(headerBox);

        Button backButton = new Button("Back");
        headerBox.getChildren().add(backButton);

        WebView webView = new WebView();
        root.setCenter(webView);
        webView.getEngine().loadContent(GuideHtml.source);

        backButton.setOnMouseClicked(_event -> {
            for (Runnable listener : backButtonListeners) {
                listener.run();
            }
        });
    }

    public void onBackButtonClicked(Runnable listener) {
        backButtonListeners.add(listener);
    }

}