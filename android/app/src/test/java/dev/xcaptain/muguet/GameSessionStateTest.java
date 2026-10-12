package dev.xcaptain.muguet;

import static org.junit.Assert.*;
import java.util.Set;
import org.junit.Test;

public class GameSessionStateTest {
    @Test public void launchOnlyOnceAndStopClosesSession() {
        GameSessionState state = new GameSessionState();
        assertTrue(state.beginLaunch());
        assertFalse(state.beginLaunch());
        state.stop();
        assertFalse(state.beginLaunch());
        assertFalse(state.acceptsInput());
    }

    @Test public void focusLossReleasesEveryHeldControl() {
        GameSessionState state = new GameSessionState();
        assertTrue(state.beginLaunch());
        state.press("KeyW");
        state.press("Space");
        assertEquals(Set.of("KeyW", "Space"), state.releaseAll());
        assertTrue(state.releaseAll().isEmpty());
        assertTrue(state.acceptsInput());
        state.stop();
        state.press("Escape");
        assertTrue(state.releaseAll().isEmpty());
    }
}
