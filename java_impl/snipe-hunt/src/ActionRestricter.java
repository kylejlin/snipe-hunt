/**
 * During the interactive tutorial, the user should only be able to take the
 * actions we want them to take.
 * 
 * To implement this, <code>PassAndPlayScreen</code> takes an
 * <code>ActionRestricter</code> as a parameter, which it uses to determine
 * whether to allow an action or not.
 */
public interface ActionRestricter {
    boolean isAuthorized(Action action);

    void onSuccessfulAction(Action action);

    void onIllegalActionAttempt(Action action);

    void onUnauthorizedActionAttempt(Action action);

    void onSuccessfulUndo();

    boolean canUserPlayFor(Player player);

    public static ActionRestricter noRestrictions() {
        return new NoRestrictions();
    }
}

class NoRestrictions implements ActionRestricter {
    public NoRestrictions() {

    }

    @Override
    public boolean isAuthorized(Action _action) {
        return true;
    }

    @Override
    public void onSuccessfulAction(Action _action) {

    }

    @Override
    public void onIllegalActionAttempt(Action action) {
    }

    @Override
    public void onSuccessfulUndo() {

    }

    @Override
    public boolean canUserPlayFor(Player player) {
        return true;
    }

    @Override
    public void onUnauthorizedActionAttempt(Action action) {
        throw new IllegalStateException("Impossible: action " + action
                + " cannot be unauthorized because NoRestrictions authorizes all actions.");
    }
}