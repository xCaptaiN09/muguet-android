package dev.xcaptain.muguet;

import java.util.ArrayList;
import java.util.List;

/** Safe-area geometry in dp, independently verified down to 440 x 320 landscape. */
final class MobileControlsLayout {
  static final class Button {
    final MobileControls.Action action;
    final float x, y, r;
    Button(MobileControls.Action action, float x, float y, float radius) {
      this.action = action;
      this.x = x;
      this.y = y;
      this.r = Math.max(24, radius);
    }
  }
  final List<Button> buttons = new ArrayList<>();
  final float infoLeft, infoTop;
  static MobileControlsLayout fromBounds(
      float measuredWidth, float measuredHeight, float left, float top, float right, float bottom) {
    return new MobileControlsLayout(
        left, top, measuredWidth - left - right, measuredHeight - top - bottom);
  }
  MobileControlsLayout(float left, float top, float width, float height) {
    float right = left + width, bottom = top + height;
    add(MobileControls.Action.PRIMARY, right - 62, bottom - 64, 40);
    add(MobileControls.Action.AIM, right - 158, bottom - 44, 31);
    add(MobileControls.Action.JUMP, right - 56, bottom - 166, 28);
    add(MobileControls.Action.ENTER, right - 142, bottom - 128, 26);
    add(MobileControls.Action.RELOAD, right - 224, bottom - 96, 24);
    MobileControls.Action[] start = {MobileControls.Action.PAUSE, MobileControls.Action.PHONE,
        MobileControls.Action.MENU, MobileControls.Action.SWITCH};
    MobileControls.Action[] end = {MobileControls.Action.WEAPON, MobileControls.Action.COVER,
        MobileControls.Action.CROUCH, MobileControls.Action.CAMERA};
    for (int i = 0; i < 4; i++) {
      add(start[i], left + 24 + i * 56, top + 32, 24);
      add(end[i], right - 24 - i * 56, top + 32, 24);
    }
    MobileControls.Action[] nav = {MobileControls.Action.NAV_LEFT, MobileControls.Action.NAV_UP,
        MobileControls.Action.NAV_DOWN, MobileControls.Action.NAV_RIGHT,
        MobileControls.Action.NAV_OK, MobileControls.Action.NAV_BACK};
    for (int i = 0; i < nav.length; i++)
      add(nav[i], left + width / 2 + (i - 2.5f) * 56, top + 92, 24);
    infoLeft = left + width / 2 - 24;
    infoTop = top + (width < 496 ? 124 : 8);
  }
  private void add(MobileControls.Action action, float x, float y, float radius) {
    buttons.add(new Button(action, x, y, radius));
  }
}
