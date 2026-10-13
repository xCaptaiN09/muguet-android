package dev.xcaptain.muguet;

import static org.junit.Assert.assertArrayEquals;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.RandomAccessFile;
import java.nio.charset.StandardCharsets;
import org.junit.Test;

public class GameLogTailTest {
    @Test public void copiesOnlyLastRequestedBytes() throws Exception {
        File source = File.createTempFile("game-log", ".txt");
        try {
            write(source, "abcdefghijklmnop", false);
            ByteArrayOutputStream output = new ByteArrayOutputStream();
            try (RandomAccessFile input = new RandomAccessFile(source, "r")) {
                GameLogTail.copy(input, output, 4);
            }
            assertArrayEquals("mnop".getBytes(StandardCharsets.UTF_8), output.toByteArray());
        } finally { source.delete(); }
    }

    @Test public void appendsDuringCopyDoNotExtendCapturedTail() throws Exception {
        File source = File.createTempFile("game-log", ".txt");
        try {
            write(source, "abcdefgh", false);
            ByteArrayOutputStream output = new ByteArrayOutputStream() {
                boolean appended;
                @Override public synchronized void write(byte[] bytes, int offset, int length) {
                    super.write(bytes, offset, length);
                    if (!appended) {
                        appended = true;
                        try { GameLogTailTest.write(source, "ijklmnop", true); }
                        catch (IOException error) { throw new RuntimeException(error); }
                    }
                }
            };
            try (RandomAccessFile input = new RandomAccessFile(source, "r")) {
                GameLogTail.copy(input, output, 5);
            }
            assertArrayEquals("defgh".getBytes(StandardCharsets.UTF_8), output.toByteArray());
        } finally { source.delete(); }
    }

    private static void write(File file, String text, boolean append) throws IOException {
        try (FileOutputStream stream = new FileOutputStream(file, append)) {
            stream.write(text.getBytes(StandardCharsets.UTF_8));
        }
    }
}
