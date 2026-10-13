package dev.xcaptain.muguet;

import java.util.*;

/** UI-thread input state. Pointer IDs (never MotionEvent indices) own roles. */
final class MobileControls {
  interface Sink {
    void key(String code, boolean pressed);
    void mouse(int button, boolean pressed);
    void horn(boolean pressed);
    void look(float dx, float dy);
  }
  enum Action {
    PRIMARY,
    AIM,
    JUMP,
    ENTER,
    RELOAD,
    WEAPON,
    COVER,
    CROUCH,
    CAMERA,
    PAUSE,
    PHONE,
    MENU,
    SWITCH,
    NAV_LEFT,
    NAV_UP,
    NAV_DOWN,
    NAV_RIGHT,
    NAV_OK,
    NAV_BACK
  }
  private enum Role { STICK, LOOK, BUTTON }
  private static final long MIN_HOLD = 120;
  static final float STICK_RADIUS = 58;
  private static final class Grip {
    final Role role;
    final Action action;
    final float ox, oy;
    float x, y;
    Grip(Role r, Action a, float x, float y) {
      role = r;
      action = a;
      ox = this.x = x;
      oy = this.y = y;
    }
  }
  private final Sink sink;
  private final Map<Integer, Grip> grips = new HashMap<>();
  private final Set<String> hardware = new HashSet<>();
  private final Set<String> emitted = new HashSet<>();
  private Set<String> desired = new HashSet<>();
  private final Map<String, Long> since = new HashMap<>();
  private boolean enabled = true, vehicle, menu, aim;
  MobileControls(Sink sink) {
    this.sink = sink;
  }
  boolean inVehicle() {
    return vehicle;
  }
  boolean menuVisible() {
    return menu;
  }
  boolean aimLatched() {
    return aim;
  }
  boolean pressed(Action action) {
    return grips.values().stream().anyMatch(g -> g.action == action);
  }
  void setEnabled(boolean value) {
    if (!value)
      releaseAll();
    enabled = value;
  }
  void setVehicle(boolean value) {
    if (vehicle != value) {
      releaseAll();
      vehicle = value;
    }
  }
  void buttonDown(int id, Action action, float x, float y, long now) {
    if (!enabled)
      return;
    if (action == Action.MENU || action == Action.PAUSE || action == Action.PHONE) {
      boolean next = action == Action.MENU ? !menu : true;
      releaseAll();
      menu = next;
    }
    if (action == Action.AIM && !vehicle)
      aim = !aim;
    grips.put(id, new Grip(Role.BUTTON, action, x, y));
    apply(now);
  }
  void stickDown(int id, float x, float y, long now) {
    if (!enabled || menu || grips.values().stream().anyMatch(g -> g.role == Role.STICK))
      return;
    grips.put(id, new Grip(Role.STICK, null, x, y));
    apply(now);
  }
  void lookDown(int id, float x, float y, long now) {
    if (enabled)
      grips.put(id, new Grip(Role.LOOK, null, x, y));
  }
  void move(int id, float x, float y, long now) {
    Grip g = grips.get(id);
    if (g == null || !enabled)
      return;
    if (g.role == Role.LOOK || (g.role == Role.BUTTON && !wheel(g.action))) {
      float dx = x - g.x, dy = y - g.y;
      float gain = 3 * (1 + Math.min(1.5f, (float) Math.hypot(dx, dy) / 12));
      sink.look(dx * gain, dy * gain);
    }
    g.x = x;
    g.y = y;
    apply(now);
  }
  void up(int id, long now) {
    grips.remove(id);
    apply(now);
  }
  void hardware(String code, boolean down, long now) {
    if (down && !enabled)
      return;
    if (down)
      hardware.add(code);
    else
      hardware.remove(code);
    apply(now);
  }
  void tick(long now) {
    if (!enabled)
      return;
    for (Grip g : grips.values())
      if (g.role == Role.BUTTON && wheel(g.action)) {
        float dx = g.x - g.ox, dy = g.y - g.oy, length = (float) Math.hypot(dx, dy);
        if (length > 10) {
          float push = 40 * Math.min(1, (length - 10) / 45);
          sink.look(dx / length * push, dy / length * push);
        }
      }
    apply(now);
  }
  private boolean wheel(Action a) {
    return a == Action.WEAPON || a == Action.SWITCH || (a == Action.COVER && vehicle);
  }
  float[] stick() {
    for (Grip g : grips.values())
      if (g.role == Role.STICK) {
        float dx = g.x - g.ox, dy = g.y - g.oy,
              k = Math.min(1, STICK_RADIUS / Math.max(1, (float) Math.hypot(dx, dy)));
        return new float[] {g.ox, g.oy, g.ox + dx * k, g.oy + dy * k};
      }
    return null;
  }
  private void apply(long now) {
    Set<String> next = new HashSet<>(hardware);
    if (aim && !vehicle)
      next.add("m2");
    for (Grip g : grips.values()) {
      if (g.role == Role.BUTTON) {
        String code = code(g.action);
        if (code != null)
          next.add(code);
      }
      if (g.role == Role.STICK && !menu) {
        float dx = g.x - g.ox, dy = g.y - g.oy, len = (float) Math.hypot(dx, dy),
              k = Math.min(1, STICK_RADIUS / Math.max(1, len));
        float nx = dx * k / STICK_RADIUS, ny = dy * k / STICK_RADIUS;
        if (nx < -.35f)
          next.add("KeyA");
        if (nx > .35f)
          next.add("KeyD");
        if (ny < (vehicle ? -.6f : -.35f))
          next.add(vehicle ? "Numpad8" : "KeyW");
        if (ny > (vehicle ? .6f : .35f))
          next.add(vehicle ? "Numpad2" : "KeyS");
        if (!vehicle && len > STICK_RADIUS * 1.35f)
          next.add("ShiftLeft");
      }
    }
    for (String code : next) {
      if (!desired.contains(code))
        since.put(code, now);
      if (emitted.add(code))
        emit(code, true);
    }
    desired = next;
    for (String code : new HashSet<>(emitted))
      if (!next.contains(code) && now - since.getOrDefault(code, now) >= MIN_HOLD) {
        emitted.remove(code);
        since.remove(code);
        emit(code, false);
      }
  }
  void releaseAll() {
    grips.clear();
    hardware.clear();
    aim = false;
    for (String code : emitted) emit(code, false);
    emitted.clear();
    desired.clear();
    since.clear();
  }
  private void emit(String code, boolean down) {
    if (code.equals("m0"))
      sink.mouse(0, down);
    else if (code.equals("m2"))
      sink.mouse(2, down);
    else if (code.equals("horn"))
      sink.horn(down);
    else
      sink.key(code, down);
  }
  private String code(Action a) {
    switch (a) {
      case PRIMARY:
        return vehicle ? "KeyW" : "m0";
      case AIM:
        return vehicle ? "KeyS" : null;
      case JUMP:
        return "Space";
      case ENTER:
        return "KeyF";
      case RELOAD:
        return vehicle ? "horn" : "KeyR";
      case WEAPON:
        return "Tab";
      case COVER:
        return "KeyQ";
      case CROUCH:
        return vehicle ? "KeyC" : "ControlLeft";
      case CAMERA:
        return "KeyV";
      case PAUSE:
        return "Escape";
      case PHONE:
      case NAV_UP:
        return "ArrowUp";
      case SWITCH:
        return "AltLeft";
      case NAV_LEFT:
        return "ArrowLeft";
      case NAV_DOWN:
        return "ArrowDown";
      case NAV_RIGHT:
        return "ArrowRight";
      case NAV_OK:
        return "Enter";
      case NAV_BACK:
        return "Backspace";
      default:
        return null;
    }
  }
  String label(Action a) {
    switch (a) {
      case PRIMARY:
        return vehicle ? "Gas" : "Fire";
      case AIM:
        return vehicle ? "Brake" : "Aim";
      case JUMP:
        return vehicle ? "Handbrake" : "Jump";
      case ENTER:
        return vehicle ? "Exit" : "Enter";
      case RELOAD:
        return vehicle ? "Horn" : "Reload";
      case COVER:
        return vehicle ? "Radio" : "Cover";
      case CROUCH:
        return vehicle ? "Look back" : "Crouch";
      case NAV_OK:
        return "Confirm";
      case NAV_BACK:
        return "Back";
      default:
        return a.name().replace("NAV_", "Menu ").toLowerCase(Locale.ROOT);
    }
  }
}
