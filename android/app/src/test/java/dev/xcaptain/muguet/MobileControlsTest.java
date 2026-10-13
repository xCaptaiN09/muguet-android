package dev.xcaptain.muguet;

import static org.junit.Assert.*;

import java.util.*;
import org.junit.Test;

public class MobileControlsTest {
  static final class Output implements MobileControls.Sink {
    final Set<String> held = new HashSet<>();
    float dx;
    public void key(String k, boolean p) {
      change(k, p);
    }
    public void mouse(int b, boolean p) {
      change("mouse" + b, p);
    }
    public void horn(boolean p) {
      change("horn", p);
    }
    public void look(float x, float y) {
      dx += x;
    }
    void change(String k, boolean p) {
      if (p)
        held.add(k);
      else
        held.remove(k);
    }
  }
  final Output out = new Output();
  final MobileControls c = new MobileControls(out);
  @Test
  public void simultaneousPointerRolesSurviveUnrelatedPointerUp() {
    c.stickDown(42, 100, 200, 0);
    c.move(42, 100, 110, 0);
    c.lookDown(8, 500, 200, 0);
    c.buttonDown(91, MobileControls.Action.PRIMARY, 0, 0, 0);
    c.move(8, 510, 200, 10);
    assertTrue(out.held.containsAll(Set.of("KeyW", "ShiftLeft", "mouse0")));
    assertTrue(out.dx > 0);
    c.up(8, 150);
    c.move(42, 180, 200, 160);
    assertTrue(out.held.containsAll(Set.of("KeyD", "mouse0")));
    c.up(91, 170);
    assertFalse(out.held.contains("mouse0"));
  }
  @Test
  public void shortTapAndRepressHaveIndependentMinimumHold() {
    c.buttonDown(1, MobileControls.Action.JUMP, 0, 0, 0);
    c.up(1, 10);
    c.tick(99);
    assertTrue(out.held.contains("Space"));
    c.buttonDown(7, MobileControls.Action.JUMP, 0, 0, 100);
    c.up(7, 110);
    c.tick(120);
    assertTrue(out.held.contains("Space"));
    c.tick(220);
    assertFalse(out.held.contains("Space"));
  }
  @Test
  public void sharedOwnersNeverReleaseEachOther() {
    c.setVehicle(true);
    c.buttonDown(1, MobileControls.Action.PRIMARY, 0, 0, 0);
    c.hardware("KeyW", true, 0);
    c.up(1, 130);
    assertTrue(out.held.contains("KeyW"));
    c.hardware("KeyW", false, 140);
    assertFalse(out.held.contains("KeyW"));
  }
  @Test
  public void contextCancelAndDisableReleaseLatchedAimHornAndPendingKeys() {
    c.buttonDown(1, MobileControls.Action.AIM, 0, 0, 0);
    c.up(1, 1);
    assertTrue(out.held.contains("mouse2"));
    c.setVehicle(true);
    assertTrue(out.held.isEmpty());
    c.buttonDown(2, MobileControls.Action.RELOAD, 0, 0, 5);
    assertTrue(out.held.contains("horn"));
    c.buttonDown(3, MobileControls.Action.JUMP, 0, 0, 5);
    c.up(3, 6);
    c.setEnabled(false);
    c.tick(500);
    assertTrue(out.held.isEmpty());
    c.buttonDown(4, MobileControls.Action.PRIMARY, 0, 0, 600);
    assertTrue(out.held.isEmpty());
  }
  @Test
  public void menuSwitchCancelsMovementAndPauseKeepsOwnAction() {
    c.stickDown(1, 100, 200, 0);
    c.move(1, 100, 120, 0);
    c.buttonDown(2, MobileControls.Action.PAUSE, 0, 0, 5);
    assertTrue(c.menuVisible());
    assertEquals(Set.of("Escape"), out.held);
    c.up(2, 6);
    c.tick(130);
    assertTrue(out.held.isEmpty());
    c.buttonDown(3, MobileControls.Action.NAV_UP, 0, 0, 200);
    assertEquals(Set.of("ArrowUp"), out.held);
    c.releaseAll();
    c.tick(1000);
    assertTrue(out.held.isEmpty());
  }
  @Test
  public void mouseAndHornShortHoldsCancelImmediatelyOnReleaseAll() {
    c.buttonDown(1, MobileControls.Action.PRIMARY, 0, 0, 0);
    c.up(1, 1);
    c.tick(119);
    assertTrue(out.held.contains("mouse0"));
    c.tick(120);
    assertTrue(out.held.isEmpty());
    c.setVehicle(true);
    c.buttonDown(2, MobileControls.Action.RELOAD, 0, 0, 200);
    c.up(2, 201);
    assertTrue(out.held.contains("horn"));
    c.releaseAll();
    c.tick(1000);
    assertTrue(out.held.isEmpty());
  }
  @Test
  public void carStickUsesPitchAndWheelDragUsesLook() {
    c.setVehicle(true);
    c.stickDown(5, 0, 0, 0);
    c.move(5, -50, -50, 0);
    assertEquals(Set.of("KeyA", "Numpad8"), out.held);
    c.buttonDown(8, MobileControls.Action.COVER, 10, 10, 0);
    c.move(8, 50, 10, 1);
    c.tick(16);
    assertTrue(out.dx > 0);
    assertTrue(out.held.contains("KeyQ"));
    c.releaseAll();
    assertTrue(out.held.isEmpty());
  }
}
