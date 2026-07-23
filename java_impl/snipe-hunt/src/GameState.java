import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;

public class GameState {
    private static CardType[] minors = { CardType.Rat1, CardType.Ox1, CardType.Rabbit1, CardType.Snake1,
            CardType.Horse1, CardType.Ram1, CardType.Monkey1, CardType.Rooster1, CardType.Dog1, CardType.Boar1,

            CardType.Squid1, CardType.Frog1,

            CardType.Rat2, CardType.Ox2, CardType.Rabbit2, CardType.Snake2, CardType.Horse2, CardType.Ram2,
            CardType.Monkey2, CardType.Rooster2, CardType.Dog2, CardType.Boar2,

            CardType.Squid2, CardType.Frog2, };
    private static CardType[] majors = { CardType.Tiger1, CardType.Dragon1, CardType.Fish1, CardType.Elephant1,
            CardType.Tiger2, CardType.Dragon2, CardType.Fish2, CardType.Elephant2, };

    public ArrayList<Card>[] initialBoard;
    public ArrayList<Card>[] currentBoard;
    public Player turn;
    public ArrayList<Action> actions;
    public Action firstAnimalStep;

    public static GameState random() {
        ArrayList<Card>[] initialBoard = getRandomBoard();
        return new GameState(initialBoard);
    }

    private GameState(ArrayList<Card>[] initialBoard) {
        this.initialBoard = initialBoard;
        currentBoard = initialBoard;
        turn = Player.Beta;
        actions = new ArrayList<>();
        firstAnimalStep = null;
    }

    private GameState(ArrayList<Card>[] initialBoard, ArrayList<Card>[] currentBoard, Player turn,
            ArrayList<Action> actions, Action firstAnimalStep) {
        this.initialBoard = initialBoard;
        this.currentBoard = currentBoard;
        this.turn = turn;
        this.actions = actions;
        this.firstAnimalStep = firstAnimalStep;
    }

    private static ArrayList<Card>[] getRandomBoard() {
        @SuppressWarnings({ "unchecked", "rawtypes" })
        ArrayList<Card>[] board = new ArrayList[] { new ArrayList(), new ArrayList(), new ArrayList(), new ArrayList(),
                new ArrayList(), new ArrayList(), new ArrayList(), new ArrayList(), };

        CardType[][] decks = getDecks();
        CardType[] alphaDeck = decks[0];
        CardType[] betaDeck = decks[1];

        board[Row.AlphaReserve].add(new Card(alphaDeck[0], Player.Alpha));

        board[Row.Rank1].add(new Card(alphaDeck[1], Player.Alpha));
        board[Row.Rank1].add(new Card(CardType.AlphaSnipe, Player.Alpha));
        board[Row.Rank1].add(new Card(alphaDeck[2], Player.Alpha));

        board[Row.Rank2].add(new Card(alphaDeck[3], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[4], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[5], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[6], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[7], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[8], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[9], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[10], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[11], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[12], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[13], Player.Alpha));
        board[Row.Rank2].add(new Card(alphaDeck[14], Player.Alpha));

        board[Row.Rank3].add(new Card(alphaDeck[15], Player.Alpha));

        board[Row.Rank4].add(new Card(betaDeck[15], Player.Beta));

        board[Row.Rank5].add(new Card(betaDeck[14], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[13], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[12], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[11], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[10], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[9], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[8], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[7], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[6], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[5], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[4], Player.Beta));
        board[Row.Rank5].add(new Card(betaDeck[3], Player.Beta));

        board[Row.Rank6].add(new Card(betaDeck[2], Player.Beta));
        board[Row.Rank6].add(new Card(CardType.BetaSnipe, Player.Beta));
        board[Row.Rank6].add(new Card(betaDeck[1], Player.Beta));

        board[Row.BetaReserve].add(new Card(betaDeck[0], Player.Beta));

        return board;
    }

    private static CardType[][] getDecks() {
        shuffle(minors);
        shuffle(majors);

        CardType[] alphaDeck = { minors[0], minors[1], minors[2], minors[3], minors[4], minors[5], minors[6], minors[7],
                minors[8], minors[9], minors[10], minors[11], majors[0], majors[1], majors[2], majors[3] };
        CardType[] betaDeck = { minors[12], minors[13], minors[14], minors[15], minors[16], minors[17], minors[18],
                minors[19], minors[20], minors[21], minors[22], minors[23], majors[4], majors[5], majors[6],
                majors[7] };

        shuffle(alphaDeck);
        shuffle(betaDeck);

        return new CardType[][] { alphaDeck, betaDeck };
    }

    private static <T> void shuffle(T[] arr) {
        final int SHUFFLE_TIMES = 512;

        for (int n = 0; n < SHUFFLE_TIMES; n++) {
            for (int i = arr.length - 1; i >= 1; i--) {
                int j = randInt(i + 1);
                T temp = arr[i];
                arr[i] = arr[j];
                arr[j] = temp;
            }
        }
    }

    private static int randInt(int exclMax) {
        return (int) Math.floor(Math.random() * exclMax);
    }

    public static GameState fromInitialBoardSpec(CardType[][] spec) {
        HashSet<CardType> alreadyUsed = new HashSet<>();

        @SuppressWarnings("unchecked")
        ArrayList<Card>[] initialBoard = new ArrayList[spec.length];

        for (int location : Row.ALL) {
            ArrayList<Card> row = new ArrayList<>();
            initialBoard[location] = row;

            Player allegiance = location <= Row.Rank3 ? Player.Alpha : Player.Beta;

            CardType[] rowSpec = spec[location];

            for (CardType type : rowSpec) {
                if (alreadyUsed.contains(type)) {
                    throw new IllegalArgumentException(type + " appears more than once in initial board spec.");
                }
                alreadyUsed.add(type);

                row.add(new Card(type, allegiance));
            }
        }

        return new GameState(initialBoard);
    }

    public boolean isGameOver() {
        return getWinner() != null;
    }

    public Player getWinner() {
        if (didAlphaCaptureBeta()) {
            return Player.Alpha;
        }

        if (didBetaCaptureAlpha()) {
            return Player.Beta;
        }

        // TODO check for out-of-moves situation

        return null;
    }

    private boolean didAlphaCaptureBeta() {
        return currentBoard[Row.AlphaReserve].stream().anyMatch(card -> card.type == CardType.BetaSnipe);
    }

    private boolean didBetaCaptureAlpha() {
        return currentBoard[Row.BetaReserve].stream().anyMatch(card -> card.type == CardType.AlphaSnipe);
    }

    public boolean isInReserve(final CardType type) {
        return currentBoard[Row.AlphaReserve].stream().anyMatch(card -> card.type == type)
                || currentBoard[Row.BetaReserve].stream().anyMatch(card -> card.type == type);
    }

    /**
     * Returns a <code>GameState</code> if the action can be legally applied,
     * otherwise returns a <code>IllegalGameStateUpdate</code>.
     * 
     * @param action
     * @return
     */
    public Object tryPerform(Action action) {
        IllegalGameStateUpdate reason = getReasonWhyActionIsIllegal(action);

        if (reason != null) {
            return reason;
        }

        return forcePerform(action);
    }

    public IllegalGameStateUpdate getReasonWhyActionIsIllegal(Action action) {
        if (isEitherSnipeCaptured()) {
            return IllegalGameStateUpdate.SnipeAlreadyCaptured;
        }

        if (action.actionType == Action.SnipeStep) {
            if (firstAnimalStep != null) {
                return IllegalGameStateUpdate.AlreadyMovedAnimal;
            }

            CardType snipe = snipeOf(turn);
            if (!isInRange(snipe, turn, action.destination)) {
                return IllegalGameStateUpdate.StepDestinationOutOfRange;
            }

            int start = getCardLocation(snipe);
            if (currentBoard[start].size() < 2) {
                return IllegalGameStateUpdate.CannotEmptyRowWithoutImmediatelyWinning;
            }

            return null;
        } else if (action.actionType == Action.Drop) {
            if (firstAnimalStep != null) {
                return IllegalGameStateUpdate.AlreadyMovedAnimal;
            }

            ArrayList<Card> reserve = turn == Player.Alpha ? currentBoard[Row.AlphaReserve]
                    : currentBoard[Row.BetaReserve];
            if (!reserve.stream().anyMatch(card -> card.type == action.dropped)) {
                return IllegalGameStateUpdate.DroppedAnimalNotInReserve;
            }

            if (reserve.size() < 2) {
                return IllegalGameStateUpdate.CannotEmptyReserve;
            }

            if (action.dropped.canRetreat() && ((turn == Player.Alpha && action.destination >= Row.Rank5)
                    || (turn == Player.Beta && action.destination <= Row.Rank2))) {
                return IllegalGameStateUpdate.CannotDropRetreaterOnEnemysBackTwoRanks;
            }

            return null;
        } else {
            int location = getCardLocation(action.moved);

            if (Row.isReserve(location)) {
                return IllegalGameStateUpdate.MovedCardInReserve;
            }

            if (!currentBoard[location].stream().filter(card -> card.allegiance == turn)
                    .anyMatch(card -> card.type == action.moved)) {
                return IllegalGameStateUpdate.NotYourAnimal;
            }

            if (!isInRange(action.moved, turn, action.destination)) {
                return IllegalGameStateUpdate.StepDestinationOutOfRange;
            }

            if (firstAnimalStep != null && firstAnimalStep.moved == action.moved) {
                return IllegalGameStateUpdate.CannotMoveSameAnimalTwice;
            }

            ArrayList<Card> start = currentBoard[location];
            ArrayList<Card> destination = currentBoard[action.destination];

            if (start.size() < 2) {
                boolean immediatelyWins = CardProperties.doesStepActivateTriplet(action.moved, destination)
                        && destination.stream().anyMatch(card -> card.type == enemySnipeOf(turn));
                if (!immediatelyWins) {
                    return IllegalGameStateUpdate.CannotEmptyRowWithoutImmediatelyWinning;
                }
            } else {
                boolean capturesOwnSnipeWithoutCapturingEnemys = CardProperties.doesStepActivateTriplet(action.moved,
                        destination) && destination.stream().anyMatch(card -> card.type == snipeOf(turn))
                        && !destination.stream().anyMatch(card -> card.type == enemySnipeOf(turn));
                if (capturesOwnSnipeWithoutCapturingEnemys) {
                    return IllegalGameStateUpdate.CannotCaptureOwnSnipeWithoutAlsoCapturingOpponents;
                }
            }

            return null;

        }
    }

    private boolean isEitherSnipeCaptured() {
        return didAlphaCaptureBeta() || didBetaCaptureAlpha();
    }

    private int getCardLocation(CardType type) {
        for (int location : Row.ALL) {
            if (currentBoard[location].stream().anyMatch(card -> card.type == type)) {
                return location;
            }
        }

        throw new Error("Impossible: Cannot find location of " + type);
    }

    private ArrayList<Card> getActivePlayersReserve() {
        return turn == Player.Alpha ? currentBoard[Row.AlphaReserve] : currentBoard[Row.BetaReserve];
    }

    public GameState forcePerform(Action action) {
        if (action.actionType == Action.SnipeStep) {
            return forcePerformSnipeStep(action);
        } else if (action.actionType == Action.Drop) {
            return forcePerformDrop(action);
        } else {
            return forcePerformAnimalStep(action);
        }
    }

    private GameState forcePerformSnipeStep(Action action) {
        int snipeLocation = getCardLocation(snipeOf(turn));

        GameState newState = cloneSelf();

        ArrayList<Card> start = newState.currentBoard[snipeLocation];
        Card removed = remove(start, snipeOf(turn));

        ArrayList<Card> destination = newState.currentBoard[action.destination];
        destination.add(removed);

        newState.actions.add(action);
        newState.turn = turn.opponent();

        return newState;
    }

    public GameState cloneSelf() {

        @SuppressWarnings({ "unchecked" })
        ArrayList<Card>[] newInitialBoard = new ArrayList[] { deepCloneCardList(initialBoard[0]),
                deepCloneCardList(initialBoard[1]), deepCloneCardList(initialBoard[2]),
                deepCloneCardList(initialBoard[3]), deepCloneCardList(initialBoard[4]),
                deepCloneCardList(initialBoard[5]), deepCloneCardList(initialBoard[6]),
                deepCloneCardList(initialBoard[7]), };

        @SuppressWarnings({ "unchecked" })
        ArrayList<Card>[] newCurrentBoard = new ArrayList[] { deepCloneCardList(currentBoard[0]),
                deepCloneCardList(currentBoard[1]), deepCloneCardList(currentBoard[2]),
                deepCloneCardList(currentBoard[3]), deepCloneCardList(currentBoard[4]),
                deepCloneCardList(currentBoard[5]), deepCloneCardList(currentBoard[6]),
                deepCloneCardList(currentBoard[7]), };

        @SuppressWarnings("unchecked")
        ArrayList<Action> newPlies = (ArrayList<Action>) actions.clone();

        return new GameState(newInitialBoard, newCurrentBoard, turn, newPlies, firstAnimalStep);
    }

    public static ArrayList<Card> deepCloneCardList(ArrayList<Card> list) {
        ArrayList<Card> clonedList = new ArrayList<>();
        for (Card card : list) {
            clonedList.add(card.safeClone());
        }
        return clonedList;
    }

    private static Card remove(ArrayList<Card> cards, CardType type) {
        int index = -1;
        for (int i = 0; i < cards.size(); i++) {
            if (cards.get(i).type == type) {
                index = i;
                break;
            }
        }

        return cards.remove(index);
    }

    private GameState forcePerformDrop(Action action) {
        GameState newState = cloneSelf();

        ArrayList<Card> reserve = newState.getActivePlayersReserve();
        Card removed = remove(reserve, action.dropped);

        newState.currentBoard[action.destination].add(removed);
        newState.actions.add(action);
        newState.turn = newState.turn.opponent();
        return newState;
    }

    private GameState forcePerformAnimalStep(Action action) {
        int movedLocation = getCardLocation(action.moved);

        GameState newState = cloneSelf();

        ArrayList<Card> start = newState.currentBoard[movedLocation];
        Card removed = remove(start, action.moved);

        ArrayList<Card> destination = newState.currentBoard[action.destination];
        if (CardProperties.doesStepActivateTriplet(action.moved, destination)) {
            ArrayList<Card> reserve = newState.getActivePlayersReserve();

            for (Card captured : destination) {
                captured.allegiance = turn;
                reserve.add(captured);
            }

            destination.clear();
            destination.add(removed);
        } else {
            destination.add(removed);
        }

        newState.actions.add(action);

        if (firstAnimalStep == null) {
            newState.firstAnimalStep = action;
        } else {
            newState.firstAnimalStep = null;
            newState.turn = newState.turn.opponent();
        }

        return newState;
    }

    private static CardType snipeOf(Player player) {
        if (player == Player.Alpha) {
            return CardType.AlphaSnipe;
        } else {
            return CardType.BetaSnipe;
        }
    }

    private static CardType enemySnipeOf(Player player) {
        if (player == Player.Alpha) {
            return CardType.BetaSnipe;
        } else {
            return CardType.AlphaSnipe;
        }
    }

    private boolean isInRange(CardType c, Player turn, int destination) {
        int location = getCardLocation(c);
        int forward = oneLocationForward(location, turn);
        int backward = oneLocationBackward(location, turn);

        return destination == forward || (c.canRetreat() && backward == destination);
    }

    private static int oneLocationForward(int location, Player turn) {
        int delta = turn == Player.Alpha ? 1 : -1;
        return location + delta;
    }

    private static int oneLocationBackward(int location, Player turn) {
        int delta = turn == Player.Alpha ? -1 : 1;
        return location + delta;
    }

    public GameState forceUndo() {
        GameState resetState = cloneSelf();
        resetState.currentBoard = resetState.initialBoard;
        resetState.turn = Player.Beta;
        resetState.actions = new ArrayList<>();
        resetState.firstAnimalStep = null;

        GameState newState = resetState.cloneSelf();
        List<Action> allButLast = actions.subList(0, actions.size() - 1);
        for (Action action : allButLast) {
            newState = newState.forcePerform(action);
        }

        return newState;
    }
}