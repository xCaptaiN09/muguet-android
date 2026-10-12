package dev.xcaptain.muguet;

import org.json.JSONException;
import org.json.JSONObject;
import org.json.JSONTokener;

import java.io.IOException;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.time.Instant;

/** Combines actual probe results without turning absent/invalid checks into success. */
final class DiagnosticReport {
    private DiagnosticReport() {}

    static JSONObject assemble(String runtimeJson, String graphicsJson) {
        JSONObject runtime = parseProbe(runtimeJson);
        JSONObject graphics = parseProbe(graphicsJson);
        try {
            return new JSONObject()
                    .put("schemaVersion", 1)
                    .put("generatedAt", Instant.now().toString())
                    .put("scope", "This build does not yet run GTA V.")
                    .put("gameCompatibilityCertified", false)
                    .put("status", "passed".equals(runtime.opt("status"))
                            && "passed".equals(graphics.opt("status"))
                            && Boolean.TRUE.equals(graphics.opt("presented")) ? "passed" : "failed")
                    .put("runtime", runtime)
                    .put("graphics", graphics);
        } catch (JSONException error) {
            throw new IllegalStateException("Could not assemble diagnostic report", error);
        }
    }

    private static JSONObject parseProbe(String json) {
        try {
            if (json == null) throw new JSONException("Native result was null");
            JsonSyntax.requireJson(json);
            JSONTokener tokens = new JSONTokener(json);
            Object value = tokens.nextValue();
            if (!(value instanceof JSONObject) || tokens.nextClean() != 0) {
                throw new JSONException("Expected exactly one JSON object");
            }
            JSONObject probe = (JSONObject) value;
            Object status = probe.opt("status");
            if (!"passed".equals(status) && !"failed".equals(status)
                    && !"pending".equals(status) && !"skipped".equals(status)) {
                throw new JSONException("Missing or invalid probe status");
            }
            return probe;
        } catch (JSONException error) {
            JSONObject failed = diagnostic("failed", "Invalid native JSON: " + error.getMessage());
            try { failed.put("rawNativeResult", json == null ? JSONObject.NULL : json); }
            catch (JSONException impossible) { throw new IllegalStateException(impossible); }
            return failed;
        }
    }

    static String failure(Throwable error) {
        return diagnostic("failed", error.toString()).toString();
    }

    static String pending(String reason) {
        return diagnostic("pending", reason).toString();
    }

    private static JSONObject diagnostic(String status, String details) {
        try { return new JSONObject().put("status", status).put("details", details); }
        catch (JSONException impossible) { throw new IllegalStateException(impossible); }
    }

    static void write(JSONObject report, OutputStream output) throws IOException {
        output.write(report.toString().getBytes(StandardCharsets.UTF_8));
        output.flush();
    }
}
