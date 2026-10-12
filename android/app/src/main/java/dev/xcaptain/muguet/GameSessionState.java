package dev.xcaptain.muguet;

import java.util.HashSet;
import java.util.Set;

/** UI-thread gate for one session and its currently held controls. */
final class GameSessionState {
    private boolean launched;
    private boolean stopped;
    private final Set<String> held = new HashSet<>();

    boolean beginLaunch() {
        if (launched || stopped) return false;
        launched = true;
        return true;
    }

    boolean acceptsInput() { return launched && !stopped; }

    void press(String code) { if (acceptsInput()) held.add(code); }

    void release(String code) { held.remove(code); }

    Set<String> releaseAll() {
        Set<String> released = new HashSet<>(held);
        held.clear();
        return released;
    }

    void stop() { stopped = true; held.clear(); }
}
