package dev.xcaptain.muguet;

import static org.junit.Assert.*;

import org.junit.Test;

public class MobileControlsColorsTest {
  @Test
  public void whiteIconsAndSmallLabelsStayReadableOnEveryBackdropAndState() {
    for (boolean pressed : new boolean[] {false, true}) {
      for (boolean selected : new boolean[] {false, true}) {
        int backing = MobileControlsColors.backing(pressed, selected);
        for (int background : new int[] {0xff000000, 0xffffffff, 0xfff4d97d, 0xff47dfff}) {
          double[] surface = composite(backing, background);
          double ratio = contrast(surface, composite(MobileControlsColors.FOREGROUND, surface));
          assertTrue("Label contrast " + ratio + ", pressed=" + pressed + ", selected=" + selected,
              ratio >= 4.5);
          assertTrue("Outline contrast",
              contrast(surface, composite(MobileControlsColors.outline(pressed, selected), surface))
                  >= 3);
        }
      }
    }
  }
  private static double[] composite(int foreground, int background) {
    return composite(foreground,
        new double[] {(background >> 16 & 255), (background >> 8 & 255), (background & 255)});
  }
  private static double[] composite(int foreground, double[] background) {
    double alpha = (foreground >>> 24) / 255.0;
    return new double[] {(foreground >> 16 & 255) * alpha + background[0] * (1 - alpha),
        (foreground >> 8 & 255) * alpha + background[1] * (1 - alpha),
        (foreground & 255) * alpha + background[2] * (1 - alpha)};
  }
  private static double linear(double color) {
    double s = color / 255;
    return s <= .04045 ? s / 12.92 : Math.pow((s + .055) / 1.055, 2.4);
  }
  private static double luminance(double[] rgb) {
    return .2126 * linear(rgb[0]) + .7152 * linear(rgb[1]) + .0722 * linear(rgb[2]);
  }
  private static double contrast(double[] a, double[] b) {
    double x = luminance(a), y = luminance(b);
    return (Math.max(x, y) + .05) / (Math.min(x, y) + .05);
  }
}
