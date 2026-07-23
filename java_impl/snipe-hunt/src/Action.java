public class Action {
    public static final int SnipeStep = 0;
    public static final int Drop = 1;
    public static final int AnimalStep = 2;

    public final int actionType;

    public final int destination;
    public final CardType dropped;
    public final CardType moved;

    public static Action snipeStep(int destination) {
        return new Action(SnipeStep, destination, null, null);
    }

    public static Action drop(CardType dropped, int destination) {
        return new Action(Drop, destination, dropped, null);
    }

    public static Action animalStep(CardType moved, int destination) {
        return new Action(AnimalStep, destination, null, moved);
    }

    private Action(int actionType, int destination, CardType dropped, CardType moved) {
        this.actionType = actionType;

        this.destination = destination;
        this.dropped = dropped;
        this.moved = moved;
    }

    public boolean equals(Action other) {
        if (other == null) {
            return false;
        }

        if (actionType != other.actionType) {
            return false;
        }

        if (actionType == Action.SnipeStep) {
            return destination == other.destination;
        } else if (actionType == Action.Drop) {
            return dropped == other.dropped && destination == other.destination;
        } else {
            return moved == other.moved && destination == other.destination;
        }
    }
}