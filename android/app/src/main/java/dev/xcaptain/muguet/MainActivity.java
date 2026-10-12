package dev.xcaptain.muguet;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Intent;
import android.graphics.Insets;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Parcel;
import android.view.Surface;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.WindowInsets;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import org.json.JSONException;
import org.json.JSONObject;

import java.io.IOException;
import java.io.OutputStream;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** A diagnostic shell; it contains no game engine or asset provider. */
public final class MainActivity extends Activity implements SurfaceHolder.Callback {
    private static final int EXPORT_REPORT = 1;
    private static final ExecutorService WORKER = Executors.newSingleThreadExecutor();
    private SurfaceView surfaceView;
    private TextView resultView;
    private Button runButton;
    private Button exportButton;
    // UI-thread state, except volatile cancellation fields read by the serial worker.
    // Workers receive snapshots and post results back to this thread.
    private boolean busy;
    private volatile boolean resumed;
    private volatile boolean destroyed;
    private volatile int surfaceGeneration;
    private int surfaceWidth;
    private int surfaceHeight;
    private String runtime = DiagnosticReport.pending("Runtime checks have not run");
    private String graphics = DiagnosticReport.pending("Vulkan presentation has not run");
    private String displayedReport;
    private String pendingExport;

    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        if (Build.VERSION.SDK_INT >= 30) getWindow().setDecorFitsSystemWindows(false);
        LinearLayout content = new LinearLayout(this);
        content.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(16);
        content.setPadding(padding, padding, padding, padding);
        content.setOnApplyWindowInsetsListener((view, insets) -> {
            if (Build.VERSION.SDK_INT >= 30) {
                Insets bars = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
                view.setPadding(padding + bars.left, padding + bars.top,
                        padding + bars.right, padding + bars.bottom);
            } else {
                view.setPadding(padding + insets.getSystemWindowInsetLeft(),
                        padding + insets.getSystemWindowInsetTop(),
                        padding + insets.getSystemWindowInsetRight(),
                        padding + insets.getSystemWindowInsetBottom());
            }
            return insets;
        });
        content.addView(label(R.string.scope, 22));
        content.addView(label(R.string.description, 16));
        content.addView(label(R.string.delivery, 14));
        surfaceView = new SurfaceView(this);
        surfaceView.setContentDescription(getString(R.string.surface_description));
        surfaceView.getHolder().addCallback(this);
        content.addView(surfaceView, new LinearLayout.LayoutParams(-1, dp(180)));
        runButton = new Button(this);
        runButton.setText(R.string.run_checks);
        runButton.setOnClickListener(view -> runChecks());
        content.addView(runButton);
        exportButton = new Button(this);
        exportButton.setText(R.string.export_report);
        exportButton.setOnClickListener(view -> chooseReportDestination());
        content.addView(exportButton);
        resultView = new TextView(this);
        resultView.setTextIsSelectable(true);
        resultView.setTextSize(13);
        resultView.setTypeface(android.graphics.Typeface.MONOSPACE);
        content.addView(resultView);
        ScrollView scroll = new ScrollView(this);
        scroll.setFillViewport(true);
        scroll.addView(content);
        setContentView(scroll);
        content.requestApplyInsets();
        if (state != null) {
            runtime = state.getString("runtime", runtime);
            // A new Activity owns a new Surface; an old frame cannot certify it.
            pendingExport = state.getString("pendingExport");
        }
        showReport();
    }

    private TextView label(int resource, int size) {
        TextView view = new TextView(this);
        view.setText(resource);
        view.setTextSize(size);
        view.setPadding(0, 0, 0, dp(12));
        return view;
    }

    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }

    private void showReport() {
        displayedReport = DiagnosticReport.assemble(runtime, graphics).toString();
        resultView.setText(displayedReport);
        runButton.setEnabled(!busy && resumed);
        runButton.setText(busy ? R.string.running : R.string.run_checks);
        exportButton.setEnabled(!busy && pendingExport == null);
    }

    private void runChecks() {
        if (busy || !resumed || destroyed) return;
        final int generation = surfaceGeneration;
        final int width = surfaceWidth;
        final int height = surfaceHeight;
        Surface snapshot = null;
        String surfaceFailure = null;
        try {
            Surface available = surfaceView.getHolder().getSurface();
            if (available != null && available.isValid() && width > 0 && height > 0) {
                // Own this reference until the serial worker finishes, even if the holder changes.
                Parcel parcel = Parcel.obtain();
                try {
                    available.writeToParcel(parcel, 0);
                    parcel.setDataPosition(0);
                    snapshot = Surface.CREATOR.createFromParcel(parcel);
                } finally { parcel.recycle(); }
            }
        } catch (RuntimeException error) {
            surfaceFailure = DiagnosticReport.failure(error);
        }
        final Surface target = snapshot;
        final String acquisitionFailure = surfaceFailure;
        busy = true;
        runtime = DiagnosticReport.pending("Runtime checks running");
        graphics = DiagnosticReport.pending("Vulkan presentation running");
        showReport();
        WORKER.execute(() -> {
            String runtimeResult;
            String graphicsResult;
            try {
                if (destroyed) return;
                try { runtimeResult = NativeBridge.runRuntimeChecks(); }
                catch (LinkageError | RuntimeException error) { runtimeResult = DiagnosticReport.failure(error); }
                if (destroyed) return;
                if (generation != surfaceGeneration || !resumed) {
                    graphicsResult = DiagnosticReport.pending("Surface changed before rendering; run checks again");
                } else if (acquisitionFailure != null) {
                    graphicsResult = acquisitionFailure;
                } else if (target == null || !target.isValid()) {
                    graphicsResult = DiagnosticReport.pending("No valid Surface; run checks again when visible");
                } else {
                    try { graphicsResult = NativeBridge.renderSurface(target, width, height); }
                    catch (LinkageError | RuntimeException error) { graphicsResult = DiagnosticReport.failure(error); }
                }
                final String completedRuntime = runtimeResult;
                final String completedGraphics = graphicsResult;
                runOnUiThread(() -> {
                    if (destroyed) return;
                    busy = false;
                    runtime = completedRuntime;
                    graphics = generation == surfaceGeneration && resumed ? completedGraphics
                            : DiagnosticReport.pending("Surface changed; run checks again to present a fresh frame");
                    showReport();
                });
            } finally {
                if (target != null) target.release();
            }
        });
    }

    private void chooseReportDestination() {
        if (busy || pendingExport != null || destroyed) return;
        pendingExport = displayedReport;
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT)
                .addCategory(Intent.CATEGORY_OPENABLE)
                .setType("application/json")
                .putExtra(Intent.EXTRA_TITLE, "muguet-diagnostic-report.json");
        try { startActivityForResult(intent, EXPORT_REPORT); }
        catch (ActivityNotFoundException error) {
            pendingExport = null;
            Toast.makeText(this, getString(R.string.no_document_picker, error.getMessage()), Toast.LENGTH_LONG).show();
        }
        showReport();
    }

    @Override protected void onActivityResult(int request, int result, Intent data) {
        super.onActivityResult(request, result, data);
        if (request != EXPORT_REPORT) return;
        String report = pendingExport;
        pendingExport = null;
        Uri destination = data == null ? null : data.getData();
        if (result != RESULT_OK || destination == null || report == null || destroyed) {
            showReport();
            return;
        }
        busy = true;
        showReport();
        WORKER.execute(() -> {
            String errorMessage = null;
            try (OutputStream output = getContentResolver().openOutputStream(destination, "wt")) {
                if (output == null) throw new IOException("Document provider returned no output stream");
                DiagnosticReport.write(new JSONObject(report), output);
            } catch (IOException | JSONException | RuntimeException error) { errorMessage = error.toString(); }
            final String completedError = errorMessage;
            runOnUiThread(() -> {
                if (destroyed) return;
                busy = false;
                showReport();
                Toast.makeText(this, completedError == null ? getString(R.string.report_exported)
                        : getString(R.string.export_failed, completedError), Toast.LENGTH_LONG).show();
            });
        });
    }

    @Override protected void onSaveInstanceState(Bundle state) {
        state.putString("runtime", runtime);
        state.putString("pendingExport", pendingExport);
        super.onSaveInstanceState(state);
    }

    @Override protected void onResume() {
        super.onResume();
        resumed = true;
        showReport();
    }

    @Override protected void onPause() {
        resumed = false;
        invalidateSurface("Activity paused; run checks again when visible");
        super.onPause();
    }

    @Override protected void onDestroy() {
        destroyed = true;
        surfaceView.getHolder().removeCallback(this);
        // Process-wide serial worker survives recreation so native calls never overlap.
        // Queued tasks observe destroyed and release their owned Surface in finally.
        super.onDestroy();
    }

    private void invalidateSurface(String reason) {
        surfaceGeneration++;
        graphics = DiagnosticReport.pending(reason);
        showReport();
    }

    @Override public void surfaceCreated(SurfaceHolder holder) {
        invalidateSurface("Surface created; run checks to present a frame");
    }

    @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        surfaceWidth = width;
        surfaceHeight = height;
        invalidateSurface("Surface changed; run checks to present a frame");
    }

    @Override public void surfaceDestroyed(SurfaceHolder holder) {
        surfaceWidth = 0;
        surfaceHeight = 0;
        invalidateSurface("Surface unavailable; graphics check pending");
    }
}
