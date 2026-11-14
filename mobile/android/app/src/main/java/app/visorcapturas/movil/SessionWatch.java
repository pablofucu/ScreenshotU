package app.visorcapturas.movil;

/** Decisión pura por encuesta del vigilante de sesión en streaming para volver al visor al caer o terminar. */
final class SessionWatch {
    enum Action {
        CONTINUE,
        ACTIVE,
        RETURN_TO_APP,
        STOP,
    }

    private boolean seenActive = false;
    private boolean sawPending = false;
    private boolean hasBaseline = false;
    private long baselineEnded = -1;
    private int fails = 0;

    Action step(boolean pollOk, String state, long ended, boolean front) {
        if (!pollOk) {
            fails++;
            if ((seenActive || sawPending) && fails >= 5) return Action.RETURN_TO_APP;
            if (!seenActive && !sawPending && fails >= 15) return Action.STOP;
            return Action.CONTINUE;
        }
        fails = 0;
        if (!hasBaseline) {
            hasBaseline = true;
            baselineEnded = ended;
        }
        if ("starting".equals(state) || "playing".equals(state)) {
            seenActive = true;
            return Action.ACTIVE;
        }
        if ("pending".equals(state)) {
            sawPending = true;
            return Action.CONTINUE;
        }
        if (seenActive || sawPending || (front && ended != baselineEnded)) {
            return Action.RETURN_TO_APP;
        }
        return front ? Action.CONTINUE : Action.STOP;
    }

    boolean wasSeen() {
        return seenActive;
    }
}
