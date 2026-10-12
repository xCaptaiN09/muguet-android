package dev.xcaptain.muguet;

import android.view.Surface;

public final class NativeBridge {
    static { System.loadLibrary("muguet_android"); }
    private NativeBridge() {}
    public static native String runRuntimeChecks();
    public static native String renderSurface(Surface surface, int width, int height);
}
