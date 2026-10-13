package dev.xcaptain.muguet;
import static org.junit.Assert.*;

import org.junit.Test;
public class MobileControlsLayoutTest {
  @Test
  public void targetsStayWithinSafeLandscapeAndDoNotCollide() {
    for (float width : new float[] {440, 480, 640, 872, 1280}) {
      MobileControlsLayout l = new MobileControlsLayout(17, 8, width, 320);
      for (MobileControlsLayout.Button b : l.buttons) {
        assertTrue(b.r >= 24);
        assertTrue(b.x - b.r >= 17);
        assertTrue(b.x + b.r <= 17 + width);
        assertTrue(b.y - b.r >= 8);
        assertTrue(b.y + b.r <= 328);
        assertTrue("Info collides with " + b.action + " at " + width,
            Math.hypot(b.x - (17 + width / 2), b.y - (l.infoTop + 24)) >= b.r + 24 + 8 - .1);
        for (MobileControlsLayout.Button other : l.buttons)
          if (b != other)
            assertTrue(b.action + " collides with " + other.action + " at " + width,
                Math.hypot(b.x - other.x, b.y - other.y) >= b.r + other.r + 8 - .1);
      }
    }
  }
  @Test
  public void measuredBoundsKeepInfoAtSafeCenterAroundCompactBreakpoint() {
    for (float safeWidth : new float[] {495, 496, 497}) {
      for (float[] insets : new float[][] {{36, 0}, {0, 36}, {36, 12}}) {
        MobileControlsLayout layout = MobileControlsLayout.fromBounds(
            safeWidth + insets[0] + insets[1], 344, insets[0], 8, insets[1], 16);
        // Activity applies this exact left/top placement with absolute LEFT/TOP gravity.
        assertEquals(insets[0] + safeWidth / 2, layout.infoLeft + 24, .01);
        assertEquals(safeWidth < 496 ? 132 : 16, layout.infoTop, .01);
        for (MobileControlsLayout.Button button : layout.buttons)
          assertTrue("Info overlaps " + button.action,
              Math.hypot(button.x - (layout.infoLeft + 24), button.y - (layout.infoTop + 24))
                  >= button.r + 24 + 8 - .1);
      }
    }
  }
  @Test
  public void compactWidthMovesOptionalInfoBelowTopRows() {
    assertEquals(132, new MobileControlsLayout(0, 8, 480, 360).infoTop, 0);
    assertEquals(16, new MobileControlsLayout(0, 8, 872, 360).infoTop, 0);
  }
}
