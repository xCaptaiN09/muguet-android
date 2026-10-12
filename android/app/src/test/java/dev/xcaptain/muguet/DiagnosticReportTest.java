package dev.xcaptain.muguet;

import org.json.JSONObject;
import org.junit.Test;
import static org.junit.Assert.*;

public class DiagnosticReportTest {
    private static final String PASS = "{\"status\":\"passed\",\"details\":\"probe complete\"}";

    private static final String GRAPHICS_PASS = "{\"status\":\"passed\",\"presented\":true,\"details\":\"frame presented\"}";

    @Test public void nativeFailureRemainsFailedEvenWhenGraphicsPasses() throws Exception {
        JSONObject report = DiagnosticReport.assemble("{\"status\":\"failed\",\"details\":\"atomic wait timed out\"}", GRAPHICS_PASS);
        assertEquals("failed", report.getString("status"));
        assertEquals("atomic wait timed out", report.getJSONObject("runtime").getString("details"));
    }

    @Test public void malformedNativeJsonBecomesFailedDiagnostic() throws Exception {
        for (String bad : new String[] {null, "not json", "[]", "{}", "{\"status\":true}", "{\"status\":\"passed\"} trailing", "null"}) {
            JSONObject report = DiagnosticReport.assemble(bad, GRAPHICS_PASS);
            assertEquals("Input: " + bad, "failed", report.getString("status"));
            assertEquals("failed", report.getJSONObject("runtime").getString("status"));
        }
    }

    @Test public void lenientJsonExtensionsCannotBecomePassedReports() throws Exception {
        for (String bad : new String[] {
                "{status:'passed'}", "{\"status\":\"passed\",}",
                "{\"status\":\"passed\",\"details\":NaN}",
                "{\"status\":\"passed\",\"details\":[1,]}",
                "{\"status\":\"passed\",\"details\":01}",
                "{\"status\":\"passed\",\"details\":\"bad\ntext\"}"}) {
            JSONObject report = DiagnosticReport.assemble(bad, GRAPHICS_PASS);
            assertEquals("Input: " + bad, "failed", report.getString("status"));
        }
    }

    @Test public void validNestedJsonValuesArePreserved() throws Exception {
        JSONObject report = DiagnosticReport.assemble(
                " {\"status\":\"passed\",\"details\":[true,false,null,-1.25e+2,{\"quoted\":\"a\\\"b\\n\\u263a\"}]} ", GRAPHICS_PASS);
        assertEquals("passed", report.getString("status"));
        assertEquals(-125.0, report.getJSONObject("runtime").getJSONArray("details").getDouble(3), 0.0);
        assertEquals("a\"b\n☺", report.getJSONObject("runtime").getJSONArray("details").getJSONObject(4).getString("quoted"));
    }

    @Test public void pendingAndSkippedGraphicsCannotPass() throws Exception {
        for (String status : new String[] {"pending", "skipped"}) {
            JSONObject report = DiagnosticReport.assemble(PASS, "{\"status\":\"" + status + "\"}");
            assertEquals("failed", report.getString("status"));
            assertFalse(report.getBoolean("gameCompatibilityCertified"));
        }
    }

    @Test public void passedGraphicsWithoutActualPresentationCannotPass() throws Exception {
        for (String graphics : new String[] {
                "{\"status\":\"passed\",\"presented\":false}",
                "{\"status\":\"passed\"}",
                "{\"status\":\"passed\",\"presented\":\"true\"}"}) {
            assertEquals("failed", DiagnosticReport.assemble(PASS, graphics).getString("status"));
        }
    }

    @Test public void successfulProbesPreserveDetailsWithoutCertifyingGame() throws Exception {
        JSONObject report = DiagnosticReport.assemble("{\"status\":\"passed\",\"checks\":[{\"name\":\"memory64\",\"status\":\"passed\",\"details\":\"two pages\"}]}", GRAPHICS_PASS);
        assertEquals("passed", report.getString("status"));
        assertEquals("two pages", report.getJSONObject("runtime").getJSONArray("checks").getJSONObject(0).getString("details"));
        assertFalse(report.getBoolean("gameCompatibilityCertified"));
        assertTrue(report.getString("scope").contains("does not yet run GTA V"));
    }

    @Test public void exportedUtf8JsonIsTheActualReport() throws Exception {
        JSONObject report = DiagnosticReport.assemble(PASS, "{\"status\":\"failed\",\"details\":\"Vulkan unavailable — GPU\"}");
        java.io.ByteArrayOutputStream destination = new java.io.ByteArrayOutputStream();
        DiagnosticReport.write(report, destination);
        JSONObject exported = new JSONObject(destination.toString(java.nio.charset.StandardCharsets.UTF_8.name()));
        assertEquals("failed", exported.getString("status"));
        assertEquals("Vulkan unavailable — GPU", exported.getJSONObject("graphics").getString("details"));
    }

    @Test public void jniLoadErrorIsRecordedAsFailure() throws Exception {
        String failure = DiagnosticReport.failure(new UnsatisfiedLinkError("libmuguet_android.so not found"));
        JSONObject report = DiagnosticReport.assemble(failure, GRAPHICS_PASS);
        assertEquals("failed", report.getString("status"));
        assertTrue(report.getJSONObject("runtime").getString("details").contains("UnsatisfiedLinkError"));
        assertTrue(report.getJSONObject("runtime").getString("details").contains("not found"));
    }
}
