import javafx.application.Application;
import javafx.stage.Stage;

public class SnipeHunt extends Application {
    private Stage stage;

    public static void main(String[] args) {
        launch(args);
    }

    @Override
    public void start(Stage stage) throws Exception {
        this.stage = stage;

        stage.setTitle("Snipe Hunt");
        stage.setResizable(false);

        stage.setWidth(1200);
        stage.setHeight(900);

        initHomeScreen();

        stage.show();

    }

    private void initHomeScreen() {
        HomeScreen screen = new HomeScreen(stage);

        screen.onPassAndPlayButtonClicked(() -> {
            initPassAndPlayScreen();
        });

        screen.onLearnButtonClicked(() -> {
            initLearnScreen();
        });

        screen.init();
    }

    private void initPassAndPlayScreen() {
        PassAndPlayScreen screen = new PassAndPlayScreen(stage, GameState.random());

        screen.onGameAborted(() -> {
            this.initHomeScreen();
        });

        screen.init();
    }

    private void initLearnScreen() {
        LearnScreen screen = new LearnScreen(stage);

        screen.onBackButtonClicked(() -> {
            this.initHomeScreen();
        });

        screen.onTutorialButtonClicked(() -> {
            initTutorialScreen();
        });

        screen.onGuideButtonClicked(() -> {
            initGuideScreen();
        });

        screen.init();
    }

    private void initTutorialScreen() {
        TutorialScreen screen = new TutorialScreen(stage);

        screen.onGameAborted(() -> {
            this.initHomeScreen();
        });

        screen.init();
    }

    private void initGuideScreen() {
        GuideScreen screen = new GuideScreen(stage);

        screen.onBackButtonClicked(() -> {
            initLearnScreen();
        });

        screen.init();
    }
}