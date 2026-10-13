package dev.xcaptain.muguet;

/** Palette shared by Canvas painting and backdrop-composited contrast checks. */
final class MobileControlsColors {
  static final int FOREGROUND = 0xffffffff;
  static int backing(boolean pressed, boolean selected) {
    return pressed ? 0xe53a5154 : selected ? 0xe5284144 : 0xcc182126;
  }
  static int outline(boolean pressed, boolean selected) {
    return pressed || selected ? 0xffffffff : 0xffe9eeee;
  }
  private MobileControlsColors() {}
}
