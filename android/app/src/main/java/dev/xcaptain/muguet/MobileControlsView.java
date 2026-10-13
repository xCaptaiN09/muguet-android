package dev.xcaptain.muguet;

import android.annotation.SuppressLint;
import android.content.Context;
import android.graphics.*;
import android.os.Bundle;
import android.os.SystemClock;
import android.view.*;
import android.view.accessibility.*;
import java.util.*;
import java.util.function.Consumer;

/** Original vector HUD inspired by GTA mobile; geometry and touch coordinates are dp. */
// The HUD is programmatic and needs its session controller; XML/tool inflation is not supported.
@SuppressLint("ViewConstructor")
final class MobileControlsView extends View {
  private static final int VIRTUAL_BASE = 100;
  private final MobileControls controls;
  private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
  private final float density;
  private float left, top, right, bottom;
  private int focused = -1, hovered = -1;
  private boolean running;
  private final List<MobileControlsLayout.Button> buttons = new ArrayList<>();
  MobileControlsView(Context context, MobileControls controls) {
    super(context);
    this.controls = controls;
    density = getResources().getDisplayMetrics().density;
    setImportantForAccessibility(IMPORTANT_FOR_ACCESSIBILITY_YES);
    setFocusable(true);
    setContentDescription(
        "Game controls. Left side movement, right side camera. Actions around screen edges.");
    setOnApplyWindowInsetsListener((v, insets) -> {
      DisplayCutout cutout = insets.getDisplayCutout();
      left = Math.max(
                 insets.getSystemWindowInsetLeft(), cutout == null ? 0 : cutout.getSafeInsetLeft())
          / density;
      top =
          Math.max(insets.getSystemWindowInsetTop(), cutout == null ? 0 : cutout.getSafeInsetTop())
          / density;
      right = Math.max(insets.getSystemWindowInsetRight(),
                  cutout == null ? 0 : cutout.getSafeInsetRight())
          / density;
      bottom = Math.max(insets.getSystemWindowInsetBottom(),
                   cutout == null ? 0 : cutout.getSafeInsetBottom())
          / density;
      layoutButtons();
      return insets;
    });
  }
  private final Runnable frame = new Runnable() {
    public void run() {
      if (!running)
        return;
      controls.tick(SystemClock.uptimeMillis());
      invalidate();
      postDelayed(this, 16);
    }
  };
  void setRunning(boolean value) {
    running = value;
    removeCallbacks(frame);
    controls.setEnabled(value);
    if (value)
      post(frame);
    invalidate();
  }
  void context(boolean vehicle) {
    if (controls.inVehicle() != vehicle) {
      controls.setVehicle(vehicle);
      invalidate();
      sendAccessibilityEvent(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED);
    }
  }
  void release() {
    controls.releaseAll();
    invalidate();
  }
  @Override
  protected void onDetachedFromWindow() {
    setRunning(false);
    super.onDetachedFromWindow();
  }
  @Override
  protected void onSizeChanged(int w, int h, int oldw, int oldh) {
    layoutButtons();
  }
  private Consumer<MobileControlsLayout> layoutListener;
  void setLayoutListener(Consumer<MobileControlsLayout> listener) {
    layoutListener = listener;
    layoutButtons();
  }
  private void layoutButtons() {
    if (getWidth() == 0 || getHeight() == 0)
      return;
    MobileControlsLayout layout = MobileControlsLayout.fromBounds(
        getWidth() / density, getHeight() / density, left, top, right, bottom);
    buttons.clear();
    buttons.addAll(layout.buttons);
    if (layoutListener != null)
      layoutListener.accept(layout);
    invalidate();
  }
  private boolean visible(MobileControlsLayout.Button b) {
    return b.action.ordinal() < MobileControls.Action.NAV_LEFT.ordinal() || controls.menuVisible();
  }
  private int hit(float x, float y) {
    for (int i = 0; i < buttons.size(); i++) {
      MobileControlsLayout.Button b = buttons.get(i);
      if (visible(b) && Math.hypot(x - b.x, y - b.y) <= b.r)
        return i;
    }
    return -1;
  }
  @Override
  public boolean onTouchEvent(MotionEvent event) {
    if (!running)
      return true;
    int action = event.getActionMasked(), index = event.getActionIndex();
    long now = event.getEventTime();
    boolean menuWasVisible = controls.menuVisible();
    if (action == MotionEvent.ACTION_CANCEL) {
      release();
      return true;
    }
    if (action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN) {
      int id = event.getPointerId(index);
      float x = event.getX(index) / density, y = event.getY(index) / density;
      int target = hit(x, y);
      if (target >= 0)
        controls.buttonDown(id, buttons.get(target).action, x, y, now);
      else if (x < left + (getWidth() / density - left - right) * .4f && y > top + 70
          && !controls.menuVisible())
        controls.stickDown(id, x, y, now);
      else
        controls.lookDown(id, x, y, now);
    } else if (action == MotionEvent.ACTION_MOVE) {
      for (int i = 0; i < event.getPointerCount(); i++)
        controls.move(event.getPointerId(i), event.getX(i) / density, event.getY(i) / density, now);
    } else if (action == MotionEvent.ACTION_UP || action == MotionEvent.ACTION_POINTER_UP) {
      controls.up(event.getPointerId(index), now);
      if (action == MotionEvent.ACTION_UP)
        performClick();
    }
    if (menuWasVisible != controls.menuVisible())
      sendAccessibilityEvent(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED);
    invalidate();
    return true;
  }
  @Override
  public boolean performClick() {
    super.performClick();
    return true;
  }
  @Override
  protected void onDraw(Canvas canvas) {
    super.onDraw(canvas);
    canvas.save();
    canvas.scale(density, density);
    float[] stick = controls.stick();
    float hx = left + 98, hy = getHeight() / density - bottom - 88;
    circle(canvas, stick == null ? hx : stick[0], stick == null ? hy : stick[1], 58, false, false);
    circle(canvas, stick == null ? hx : stick[2], stick == null ? hy : stick[3], 26, false, true);
    for (int i = 0; i < buttons.size(); i++) {
      MobileControlsLayout.Button b = buttons.get(i);
      if (!visible(b))
        continue;
      boolean toggled = (b.action == MobileControls.Action.AIM && controls.aimLatched())
          || (b.action == MobileControls.Action.MENU && controls.menuVisible());
      circle(canvas, b.x, b.y, b.r, controls.pressed(b.action), toggled || focused == i);
      canvas.save();
      canvas.translate(b.x, b.y - 3);
      icon(canvas, b.action);
      canvas.restore();
      // Labels complement authored icons; never substitute glyphs for them.
      if (i < 5 || b.action == MobileControls.Action.MENU) {
        paint.setStyle(Paint.Style.FILL);
        paint.setColor(MobileControlsColors.FOREGROUND);
        paint.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        paint.setTextAlign(Paint.Align.CENTER);
        paint.setTextSize(9 * getResources().getDisplayMetrics().scaledDensity / density);
        canvas.drawText(controls.label(b.action), b.x, b.y + b.r - 8, paint);
      }
    }
    canvas.restore();
  }
  private void circle(Canvas c, float x, float y, float r, boolean pressed, boolean selected) {
    paint.setStyle(Paint.Style.FILL);
    paint.setColor(MobileControlsColors.backing(pressed, selected));
    c.drawCircle(x, y, r, paint);
    paint.setStyle(Paint.Style.STROKE);
    paint.setStrokeWidth(pressed ? 3 : selected ? 2 : 1.3f);
    paint.setColor(MobileControlsColors.outline(pressed, selected));
    c.drawCircle(x, y, r - 1, paint);
  }
  private void line(Canvas c, float... xy) {
    Path p = new Path();
    p.moveTo(xy[0], xy[1]);
    for (int i = 2; i < xy.length; i += 2) p.lineTo(xy[i], xy[i + 1]);
    c.drawPath(p, paint);
  }
  private void icon(Canvas c, MobileControls.Action a) {
    paint.setStyle(Paint.Style.STROKE);
    paint.setColor(MobileControlsColors.FOREGROUND);
    paint.setStrokeWidth(2);
    paint.setStrokeCap(Paint.Cap.ROUND);
    paint.setStrokeJoin(Paint.Join.ROUND);
    switch (a) {
      case PRIMARY:
        if (controls.inVehicle()) {
          c.drawRoundRect(-10, -13, 10, 10, 3, 3, paint);
          line(c, -6, -7, 6, -7);
          line(c, -6, -2, 6, -2);
          line(c, -6, 3, 6, 3);
        } else {
          c.drawCircle(0, -2, 13, paint);
          c.drawCircle(0, -2, 5, paint);
          line(c, -18, -2, -9, -2);
          line(c, 9, -2, 18, -2);
          line(c, 0, -20, 0, -11);
          line(c, 0, 7, 0, 16);
        }
        break;
      case AIM:
        if (controls.inVehicle()) {
          c.drawCircle(0, -3, 12, paint);
          line(c, -5, -10, -5, 4);
          line(c, 5, -10, 5, 4);
        } else {
          line(c, -13, -7, 0, -14, 13, -7, 13, 3, 0, 10, -13, 3, -13, -7);
          c.drawCircle(0, -2, 4, paint);
        }
        break;
      case JUMP:
        if (controls.inVehicle()) {
          c.drawCircle(0, -2, 11, paint);
          line(c, -16, -10, -16, 6);
          line(c, 16, -10, 16, 6);
          line(c, -4, -8, 4, 4);
        } else {
          line(c, -12, -2, 0, -14, 12, -2);
          line(c, 0, -14, 0, 10);
          line(c, -10, 10, 10, 10);
        }
        break;
      case ENTER:
        c.drawRect(1, -13, 12, 11, paint);
        line(c, -14, -1, 7, -1, 2, -6);
        line(c, 7, -1, 2, 4);
        break;
      case RELOAD:
        if (controls.inVehicle()) {
          line(c, -13, -3, -6, -3, 5, -11, 5, 9, -6, 1, -13, 1, -13, -3);
          line(c, 10, -7, 14, -3, 14, 3, 10, 7);
        } else {
          c.drawArc(-12, -14, 12, 10, 35, 285, false, paint);
          line(c, 13, -7, 13, 1, 5, -1);
        }
        break;
      case PAUSE:
        line(c, -5, -10, -5, 10);
        line(c, 5, -10, 5, 10);
        break;
      case PHONE:
        c.drawRoundRect(-7, -12, 7, 12, 2, 2, paint);
        line(c, -2, 8, 2, 8);
        break;
      case MENU:
        for (int y = -8; y <= 8; y += 8) {
          line(c, -11, y, 11, y);
        }
        break;
      case SWITCH:
        c.drawCircle(-4, -6, 4, paint);
        c.drawArc(-13, 0, 5, 16, 190, 160, false, paint);
        line(c, 5, -10, 12, -10, 12, 5, 8, 1);
        break;
      case WEAPON:
        line(c, -13, -6, 13, -6, 13, 0, 1, 0, -3, 10, -9, 10, -6, 0, -13, 0, -13, -6);
        break;
      case COVER:
        if (controls.inVehicle()) {
          c.drawRoundRect(-12, -6, 12, 11, 2, 2, paint);
          c.drawCircle(5, 3, 4, paint);
          line(c, -10, -6, -4, -14);
          line(c, -7, 0, -3, 0);
        } else {
          line(c, 5, -13, 5, 13, 14, 13);
          c.drawCircle(-5, -7, 4, paint);
          line(c, -5, -3, -5, 7, 2, 7);
        }
        break;
      case CROUCH:
        if (controls.inVehicle()) {
          c.drawRoundRect(-13, -9, 13, 4, 3, 3, paint);
          line(c, 0, 4, 0, 11);
          line(c, 7, -4, -7, -4, -3, -8);
          line(c, -7, -4, -3, 0);
        } else {
          c.drawCircle(0, -10, 4, paint);
          line(c, 0, -6, -6, 1, 3, 5, -4, 11, -12, 11);
          line(c, -5, 0, 9, 0);
        }
        break;
      case CAMERA:
        c.drawRoundRect(-13, -8, 13, 10, 3, 3, paint);
        c.drawCircle(0, 1, 5, paint);
        line(c, -7, -8, -4, -12, 4, -12, 7, -8);
        break;
      case NAV_LEFT:
        line(c, 5, -10, -5, 0, 5, 10);
        break;
      case NAV_UP:
        line(c, -10, 5, 0, -5, 10, 5);
        break;
      case NAV_DOWN:
        line(c, -10, -5, 0, 5, 10, -5);
        break;
      case NAV_RIGHT:
        line(c, -5, -10, 5, 0, -5, 10);
        break;
      case NAV_OK:
        line(c, -11, 0, -3, 8, 12, -9);
        break;
      case NAV_BACK:
        line(c, -2, -10, -12, 0, -2, 10);
        line(c, -12, 0, 11, 0, 11, -7);
        break;
    }
  }
  @Override
  public AccessibilityNodeProvider getAccessibilityNodeProvider() {
    return accessibility;
  }
  private final AccessibilityNodeProvider accessibility = new AccessibilityNodeProvider() {
    @Override
    public AccessibilityNodeInfo createAccessibilityNodeInfo(int id) {
      if (id == View.NO_ID) {
        AccessibilityNodeInfo n = AccessibilityNodeInfo.obtain(MobileControlsView.this);
        onInitializeAccessibilityNodeInfo(n);
        for (int i = 0; i < buttons.size(); i++)
          if (visible(buttons.get(i)))
            n.addChild(MobileControlsView.this, VIRTUAL_BASE + i);
        return n;
      }
      int index = id - VIRTUAL_BASE;
      if (index < 0 || index >= buttons.size() || !visible(buttons.get(index)))
        return null;
      MobileControlsLayout.Button b = buttons.get(index);
      AccessibilityNodeInfo n = AccessibilityNodeInfo.obtain();
      n.setSource(MobileControlsView.this, id);
      n.setParent(MobileControlsView.this);
      n.setPackageName(getContext().getPackageName());
      n.setClassName("android.widget.Button");
      n.setContentDescription(controls.label(b.action));
      n.setEnabled(running);
      n.setVisibleToUser(isShown());
      n.setClickable(true);
      n.setFocusable(true);
      n.setSelected((b.action == MobileControls.Action.AIM && controls.aimLatched())
          || (b.action == MobileControls.Action.MENU && controls.menuVisible()));
      n.setBoundsInParent(new Rect((int) ((b.x - b.r) * density), (int) ((b.y - b.r) * density),
          (int) ((b.x + b.r) * density), (int) ((b.y + b.r) * density)));
      int[] at = new int[2];
      getLocationOnScreen(at);
      Rect screen =
          new Rect((int) ((b.x - b.r) * density) + at[0], (int) ((b.y - b.r) * density) + at[1],
              (int) ((b.x + b.r) * density) + at[0], (int) ((b.y + b.r) * density) + at[1]);
      n.setBoundsInScreen(screen);
      n.addAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_CLICK);
      n.addAction(focused == index
              ? AccessibilityNodeInfo.AccessibilityAction.ACTION_CLEAR_ACCESSIBILITY_FOCUS
              : AccessibilityNodeInfo.AccessibilityAction.ACTION_ACCESSIBILITY_FOCUS);
      n.setAccessibilityFocused(focused == index);
      return n;
    }
    @Override
    public boolean performAction(int id, int action, Bundle args) {
      int i = id - VIRTUAL_BASE;
      if (i < 0 || i >= buttons.size() || !visible(buttons.get(i)))
        return false;
      if (action == AccessibilityNodeInfo.ACTION_CLICK && running) {
        long now = SystemClock.uptimeMillis();
        controls.buttonDown(
            -1000 - i, buttons.get(i).action, buttons.get(i).x, buttons.get(i).y, now);
        controls.up(-1000 - i, now);
        sendAccessibilityEvent(AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED);
        announceForAccessibility(controls.label(buttons.get(i).action));
        invalidate();
        return true;
      }
      if (action == AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS) {
        focused = i;
        virtualEvent(i, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED);
        invalidate();
        return true;
      }
      if (action == AccessibilityNodeInfo.ACTION_CLEAR_ACCESSIBILITY_FOCUS) {
        focused = -1;
        virtualEvent(i, AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED);
        invalidate();
        return true;
      }
      return false;
    }
  };
  private void virtualEvent(int i, int type) {
    AccessibilityEvent e = AccessibilityEvent.obtain(type);
    e.setSource(this, VIRTUAL_BASE + i);
    e.setPackageName(getContext().getPackageName());
    e.setClassName("android.widget.Button");
    e.setContentDescription(controls.label(buttons.get(i).action));
    getParent().requestSendAccessibilityEvent(this, e);
  }
  @Override
  public boolean onHoverEvent(MotionEvent event) {
    AccessibilityManager manager =
        (AccessibilityManager) getContext().getSystemService(Context.ACCESSIBILITY_SERVICE);
    if (manager.isTouchExplorationEnabled()) {
      int target = event.getActionMasked() == MotionEvent.ACTION_HOVER_EXIT
          ? -1
          : hit(event.getX() / density, event.getY() / density);
      if (target != hovered) {
        if (hovered >= 0)
          virtualEvent(hovered, AccessibilityEvent.TYPE_VIEW_HOVER_EXIT);
        hovered = target;
        if (target >= 0)
          virtualEvent(target, AccessibilityEvent.TYPE_VIEW_HOVER_ENTER);
      }
      return target >= 0;
    }
    return super.onHoverEvent(event);
  }
}
