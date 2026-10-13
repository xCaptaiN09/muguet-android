package dev.xcaptain.muguet;

import android.content.Context;
import android.view.Surface;

public final class NativeBridge {
    static { System.loadLibrary("muguet_android"); }
    private NativeBridge() {}
    public static native String runRuntimeChecks();
    public static native String renderSurface(Surface surface, int width, int height);
    public static native String initializeGame(Context context, String logPath);
    public static native boolean verifyGameCode(String path);
    public static native String startGame(Surface surface, String gameRoot, String codePath,
            String savesDir, String cacheDir, int width, int height);
    public static native String gameStatus();
    public static native void pauseGame(boolean paused);
    public static native void gameKey(String code, boolean pressed);
    public static native void gameMouseButton(int button, boolean pressed);
    public static native void gameTouchHorn(boolean pressed);
    public static native void gameLook(float dx, float dy);
}
