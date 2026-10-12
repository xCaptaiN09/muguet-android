package dev.xcaptain.muguet;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Process;
import android.util.Log;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.widget.Button;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import org.json.JSONObject;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** One visible game session per isolated :game process. */
public final class GameActivity extends Activity implements SurfaceHolder.Callback {
    private static final String TAG = "MuguetGame";
    private static final int WIDTH = 960, HEIGHT = 432;
    private static final int LOG_EXPORT_BYTES = 2 * 1024 * 1024;
    private final Handler ui = new Handler(Looper.getMainLooper());
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private final GameSessionState state = new GameSessionState();
    private SurfaceView surfaceView;
    private TextView statusView;
    private boolean resumed;
    private volatile boolean stopping;
    private float previousX, previousY;
    private File externalRoot;
    private File privateLog;

    @Override public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        externalRoot = getExternalFilesDir(null);
        privateLog = new File(getFilesDir(), "game-host.log");
        getWindow().getDecorView().setSystemUiVisibility( View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                | View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
        FrameLayout root = new FrameLayout(this);
        surfaceView = new SurfaceView(this);
        surfaceView.getHolder().setFixedSize(WIDTH, HEIGHT);
        surfaceView.getHolder().addCallback(this);
        surfaceView.setFocusableInTouchMode(true);
        surfaceView.setOnTouchListener((view, event) -> {
            int action = event.getActionMasked();
            if (action == MotionEvent.ACTION_DOWN) {
                previousX = event.getX(); previousY = event.getY(); return true;
            }
            if (action == MotionEvent.ACTION_MOVE && state.acceptsInput()) {
                float dx = event.getX() - previousX, dy = event.getY() - previousY;
                previousX = event.getX(); previousY = event.getY();
                worker.execute(() -> NativeBridge.gameLook(dx, dy));
                return true;
            }
            return true;
        });
        root.addView(surfaceView, new FrameLayout.LayoutParams(-1, -1));
        statusView = new TextView(this);
        statusView.setText(R.string.game_status_initial);
        statusView.setTextColor(0xffffffff);
        statusView.setBackgroundColor(0x99000000);
        statusView.setTextSize(13);
        statusView.setPadding(12, 8, 12, 8);
        FrameLayout.LayoutParams statusParams = new FrameLayout.LayoutParams(-1, -2, Gravity.TOP);
        root.addView(statusView, statusParams);
        LinearLayout controls = new LinearLayout(this);
        controls.setOrientation(LinearLayout.HORIZONTAL);
        controls.addView(control("◀", "ArrowLeft"));
        controls.addView(control("▲", "ArrowUp"));
        controls.addView(control("▼", "ArrowDown"));
        controls.addView(control("▶", "ArrowRight"));
        controls.addView(control("Enter", "Enter"));
        controls.addView(control("Esc", "Escape"));
        controls.addView(control("Action", "KeyE"));
        controls.addView(control("Jump", "Space"));
        FrameLayout.LayoutParams controlParams = new FrameLayout.LayoutParams(-2, -2, Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
        root.addView(controls, controlParams);
        setContentView(root);
        surfaceView.requestFocus();
    }

    private Button control(String label, String code) {
        Button button = new Button(this);
        button.setText(label);
        button.setTextSize(12);
        button.setMinWidth(0);
        button.setOnTouchListener((view, event) -> {
            if (event.getActionMasked() == MotionEvent.ACTION_DOWN) { sendKey(code, true); return true; }
            if (event.getActionMasked() == MotionEvent.ACTION_UP || event.getActionMasked() == MotionEvent.ACTION_CANCEL) {
                sendKey(code, false); return true;
            }
            return true;
        });
        return button;
    }

    private void sendKey(String code, boolean pressed) {
        if (pressed) {
            if (!resumed || !state.acceptsInput()) return;
            state.press(code);
        } else state.release(code);
        worker.execute(() -> NativeBridge.gameKey(code, pressed));
    }

    private void releaseControls() {
        Set<String> released = state.releaseAll();
        for (String code : released) worker.execute(() -> NativeBridge.gameKey(code, false));
    }

    @Override public boolean onKeyDown(int keyCode, KeyEvent event) {
        String code = keyName(keyCode);
        if (code != null) { if (event.getRepeatCount() == 0) sendKey(code, true); return true; }
        return super.onKeyDown(keyCode, event);
    }
    @Override public boolean onKeyUp(int keyCode, KeyEvent event) {
        String code = keyName(keyCode);
        if (code != null) { sendKey(code, false); return true; }
        return super.onKeyUp(keyCode, event);
    }
    private static String keyName(int key) {
        switch (key) {
            case KeyEvent.KEYCODE_DPAD_LEFT: return "ArrowLeft";
            case KeyEvent.KEYCODE_DPAD_RIGHT: return "ArrowRight";
            case KeyEvent.KEYCODE_DPAD_UP: return "ArrowUp";
            case KeyEvent.KEYCODE_DPAD_DOWN: return "ArrowDown";
            case KeyEvent.KEYCODE_ENTER: case KeyEvent.KEYCODE_DPAD_CENTER: return "Enter";
            case KeyEvent.KEYCODE_ESCAPE: case KeyEvent.KEYCODE_BACK: return "Escape";
            case KeyEvent.KEYCODE_SPACE: return "Space";
            case KeyEvent.KEYCODE_E: return "KeyE";
            default: return null;
        }
    }

    @Override public void onWindowFocusChanged(boolean focused) {
        super.onWindowFocusChanged(focused);
        if (!focused) { releaseControls(); worker.execute(() -> NativeBridge.pauseGame(true)); }
        else if (resumed && state.acceptsInput()) worker.execute(() -> NativeBridge.pauseGame(false));
    }

    @Override protected void onResume() { super.onResume(); resumed = true; maybeLaunch(); }
    @Override protected void onPause() {
        resumed = false;
        releaseControls();
        worker.execute(() -> NativeBridge.pauseGame(true));
        super.onPause();
    }
    @Override protected void onStop() { super.onStop(); stopGameProcess(); }
    @Override protected void onDestroy() {
        surfaceView.getHolder().removeCallback(this);
        stopGameProcess();
        super.onDestroy();
    }
    private void stopGameProcess() {
        if (stopping) return;
        stopping = true;
        releaseControls();
        state.stop();
        worker.execute(() -> {
            try { NativeBridge.pauseGame(true); exportDebug(NativeBridge.gameStatus()); }
            catch (Throwable error) { Log.e(TAG, "Final status export failed", error); }
            finally { Process.killProcess(Process.myPid()); }
        });
    }

    @Override public void surfaceCreated(SurfaceHolder holder) { maybeLaunch(); }
    @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) { maybeLaunch(); }
    @Override public void surfaceDestroyed(SurfaceHolder holder) { releaseControls(); worker.execute(() -> NativeBridge.pauseGame(true)); }

    private void maybeLaunch() {
        Surface surface = surfaceView.getHolder().getSurface();
        if (!resumed || stopping || surface == null || !surface.isValid() || !state.beginLaunch()) return;
        statusView.setText("Preparing private game code…");
        worker.execute(() -> {
            String status;
            try {
                if (externalRoot == null) throw new IOException("App external files directory is unavailable");
                status = NativeBridge.initializeGame(getApplicationContext(), privateLog.getAbsolutePath());
                if ("failed".equals(new JSONObject(status).optString("phase"))) throw new IOException(status);
                File code = new File(new File(getFilesDir(), "game-code"), "game-android.cwasm");
                if (!NativeBridge.verifyGameCode(code.getAbsolutePath())) {
                    File staged = new File(new File(externalRoot, "game-code"), "game-android.cwasm");
                    copyCodeAtomically(staged, code);
                }
                status = NativeBridge.startGame(surface,
                        new File(externalRoot, "game").getAbsolutePath(), code.getAbsolutePath(),
                        new File(getFilesDir(), "saves").getAbsolutePath(),
                        new File(getCacheDir(), "game").getAbsolutePath(), WIDTH, HEIGHT);
            } catch (Throwable error) {
                status = "{\"phase\":\"failed\",\"error\":" + JSONObject.quote(error.toString()) + "}";
            }
            publish(status);
            if (!stopping && !status.contains("\"phase\":\"failed\"")) ui.postDelayed(this::pollStatus, 750);
        });
    }

    private static void copyCodeAtomically(File source, File target) throws IOException {
        if (!source.isFile()) throw new IOException("Missing staged game code: " + source);
        if (!target.getParentFile().isDirectory() && !target.getParentFile().mkdirs())
            throw new IOException("Cannot create private game-code directory");
        File temporary = new File(target.getParentFile(), target.getName() + ".part");
        try {
            try (FileInputStream input = new FileInputStream(source);
                    FileOutputStream output = new FileOutputStream(temporary)) {
                byte[] buffer = new byte[1024 * 1024];
                int count;
                while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                output.getFD().sync();
            }
            Files.move(temporary.toPath(), target.toPath(), StandardCopyOption.ATOMIC_MOVE,
                    StandardCopyOption.REPLACE_EXISTING);
        } finally { if (temporary.exists()) temporary.delete(); }
    }

    private void pollStatus() {
        if (stopping) return;
        worker.execute(() -> {
            String status;
            try { status = NativeBridge.gameStatus(); }
            catch (Throwable error) { status = "{\"phase\":\"failed\",\"error\":" + JSONObject.quote(error.toString()) + "}"; }
            publish(status);
            if (!stopping) ui.postDelayed(this::pollStatus, 750);
        });
    }

    private void publish(String status) {
        Log.i(TAG, status);
        try { exportDebug(status); } catch (IOException error) { Log.e(TAG, "Debug export failed", error); }
        ui.post(() -> {
            if (stopping) return;
            try {
                JSONObject value = new JSONObject(status);
                String line = value.optString("phase", "unknown") + "  presents=" + value.optLong("presents")
                        + "  GPU errors=" + value.optLong("gpuErrors") + "  heap=" + value.optLong("heapBytes");
                String error = value.optString("error", "");
                statusView.setText(error.isEmpty() ? line : line + "\n" + error);
            } catch (Exception error) { statusView.setText(status); }
        });
    }

    private void exportDebug(String status) throws IOException {
        if (externalRoot == null) return;
        File snapshot = new File(externalRoot, "game-status.json");
        File temp = new File(externalRoot, "game-status.json.part");
        try (FileOutputStream out = new FileOutputStream(temp)) {
            out.write(status.getBytes(java.nio.charset.StandardCharsets.UTF_8)); out.getFD().sync();
        }
        Files.move(temp.toPath(), snapshot.toPath(), StandardCopyOption.ATOMIC_MOVE,
                StandardCopyOption.REPLACE_EXISTING);
        if (!privateLog.isFile()) return;
        File logOutput = new File(externalRoot, "game-host.log");
        try (RandomAccessFile input = new RandomAccessFile(privateLog, "r");
                FileOutputStream output = new FileOutputStream(logOutput)) {
            long length = input.length();
            input.seek(Math.max(0, length - LOG_EXPORT_BYTES));
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
        }
    }
}
