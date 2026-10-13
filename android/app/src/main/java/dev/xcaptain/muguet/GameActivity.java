package dev.xcaptain.muguet;

import android.app.Activity;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.os.Process;
import android.util.Log;
import android.view.Gravity;
import android.view.KeyEvent;
import android.os.SystemClock;
import android.view.WindowManager;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.widget.Button;
import android.widget.FrameLayout;

import android.widget.TextView;

import org.json.JSONObject;

import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

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
    private MobileControls controls;
    private MobileControlsView controlsView;
    private Button debugButton;
    private boolean debugVisible, ready, focused, surfaceReady;
    private String latestStatus;

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
        root.addView(surfaceView, new FrameLayout.LayoutParams(-1, -1));
        controls = new MobileControls(new MobileControls.Sink() {
            public void key(String code, boolean pressed) { NativeBridge.gameKey(code, pressed); }
            public void mouse(int button, boolean pressed) { NativeBridge.gameMouseButton(button, pressed); }
            public void horn(boolean pressed) { NativeBridge.gameTouchHorn(pressed); }
            public void look(float dx, float dy) { NativeBridge.gameLook(dx, dy); }
        });
        controlsView = new MobileControlsView(this, controls);
        controlsView.setRunning(false);
        root.addView(controlsView, new FrameLayout.LayoutParams(-1,-1));
        statusView = new TextView(this);
        statusView.setText(R.string.game_status_initial);
        statusView.setTextColor(0xffffffff);
        statusView.setBackgroundColor(0xdd182126);
        statusView.setTextSize(16);
        statusView.setPadding(dp(20),dp(16),dp(20),dp(16));
        statusView.setGravity(Gravity.CENTER);
        FrameLayout.LayoutParams statusParams = new FrameLayout.LayoutParams(-2,-2,Gravity.CENTER);
        statusParams.leftMargin=dp(72); statusParams.rightMargin=dp(72);
        root.addView(statusView, statusParams);
        debugButton = new Button(this);
        debugButton.setText(R.string.game_info); debugButton.setTextSize(10);
        debugButton.setContentDescription(getString(R.string.game_info_description));
        debugButton.setMinWidth(0); debugButton.setMinimumWidth(0);
        debugButton.setMinHeight(0); debugButton.setMinimumHeight(0);
        debugButton.setPadding(0,0,0,0); debugButton.setAlpha(.7f);
        // Canvas game controls use physical left/right coordinates in every locale.
        @android.annotation.SuppressLint("RtlHardcoded")
        FrameLayout.LayoutParams debugParams = new FrameLayout.LayoutParams(dp(48),dp(48),Gravity.TOP|Gravity.LEFT);
        debugParams.topMargin=dp(8); root.addView(debugButton,debugParams);
        debugButton.setOnClickListener(v -> { debugVisible=!debugVisible; renderStatus(); });
        controlsView.setLayoutListener(layout -> {
            float density=getResources().getDisplayMetrics().density;
            int leftMargin=Math.round(layout.infoLeft*density);
            int topMargin=Math.round(layout.infoTop*density);
            if(debugParams.leftMargin!=leftMargin || debugParams.topMargin!=topMargin) {
                debugParams.leftMargin=leftMargin;
                debugParams.topMargin=topMargin;
                debugButton.setLayoutParams(debugParams);
            }
        });
        setContentView(root);
        surfaceView.requestFocus();
    }

    private int dp(int value) { return Math.round(value*getResources().getDisplayMetrics().density); }
    private void sendKey(String code, boolean pressed) {
        controls.hardware(code,pressed,SystemClock.uptimeMillis());
    }
    private void releaseControls() { if(controlsView!=null) controlsView.setRunning(false); }
    private void updateInput() {
        boolean active=resumed && focused && surfaceReady && ready && !stopping;
        controlsView.setRunning(active);
        NativeBridge.pauseGame(!active);
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
            case KeyEvent.KEYCODE_ESCAPE: return "Escape";
            case KeyEvent.KEYCODE_SPACE: return "Space";
            case KeyEvent.KEYCODE_E: return "KeyE";
            case KeyEvent.KEYCODE_TAB: return "Tab";
            case KeyEvent.KEYCODE_DEL: return "Backspace";
            case KeyEvent.KEYCODE_SHIFT_LEFT: return "ShiftLeft";
            case KeyEvent.KEYCODE_CTRL_LEFT: return "ControlLeft";
            case KeyEvent.KEYCODE_ALT_LEFT: return "AltLeft";
            default:
                if (key >= KeyEvent.KEYCODE_A && key <= KeyEvent.KEYCODE_Z)
                    return "Key" + (char)('A' + key-KeyEvent.KEYCODE_A);
                if (key >= KeyEvent.KEYCODE_NUMPAD_0 && key <= KeyEvent.KEYCODE_NUMPAD_9)
                    return "Numpad" + (key-KeyEvent.KEYCODE_NUMPAD_0);
                return null;
        }
    }

    @Override public void onWindowFocusChanged(boolean focused) {
        super.onWindowFocusChanged(focused);
        this.focused=focused;
        if(controlsView!=null) updateInput();
    }

    @Override protected void onResume() {
        super.onResume(); resumed=true;
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        updateInput(); maybeLaunch();
    }
    @Override protected void onPause() {
        resumed=false; releaseControls(); NativeBridge.pauseGame(true);
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        super.onPause();
    }
    @Override protected void onStop() { releaseControls(); super.onStop(); stopGameProcess(); }
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

    @Override public void surfaceCreated(SurfaceHolder holder) { surfaceReady=true; updateInput(); maybeLaunch(); }
    @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) { surfaceReady=true; updateInput(); maybeLaunch(); }
    @Override public void surfaceDestroyed(SurfaceHolder holder) { surfaceReady=false; releaseControls(); NativeBridge.pauseGame(true); }

    private void maybeLaunch() {
        Surface surface = surfaceView.getHolder().getSurface();
        if (!resumed || stopping || surface == null || !surface.isValid() || !state.beginLaunch()) return;
        statusView.setText(R.string.game_preparing);
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
            latestStatus=status;
            renderStatus();
        });
    }

    private void renderStatus() {
        if(latestStatus==null) return;
        try {
            JSONObject value=new JSONObject(latestStatus);
            String phase=value.optString("phase","unknown");
            String error=value.isNull("error") ? "" : value.optString("error","");
            boolean failed="failed".equals(phase);
            boolean nextReady=!failed && ("running".equals(phase) || "present_requested".equals(phase));
            if(ready!=nextReady) { ready=nextReady; updateInput(); }
            controlsView.context(value.optBoolean("inVehicle",false));
            if(failed) { releaseControls(); NativeBridge.pauseGame(true); }
            boolean presented=value.optLong("presents")>0;
            statusView.setVisibility(failed || !presented || debugVisible ? View.VISIBLE : View.GONE);
            if(failed) statusView.setText(error.isEmpty()?getString(R.string.game_ended):getString(R.string.game_ended_detail,error));
            else if(debugVisible) statusView.setText(getString(R.string.game_debug,phase,value.optLong("presents"),value.optLong("gpuErrors"),value.optLong("heapBytes")));
            else statusView.setText(R.string.game_loading);
        } catch(Exception error) {
            ready=false; releaseControls(); NativeBridge.pauseGame(true);
            statusView.setVisibility(View.VISIBLE); statusView.setText(R.string.game_status_unreadable);
            Log.e(TAG,"Invalid game status",error);
        }
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
            GameLogTail.copy(input, output, LOG_EXPORT_BYTES);
        }
    }
}
