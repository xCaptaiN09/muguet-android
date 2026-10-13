package dev.xcaptain.muguet;

import java.io.IOException;
import java.io.OutputStream;
import java.io.RandomAccessFile;

/** Copies a log tail for external debugging. */
final class GameLogTail {
    private GameLogTail() {}

    static void copy(RandomAccessFile input, OutputStream output, int maxBytes) throws IOException {
        if (maxBytes < 0) throw new IllegalArgumentException("maxBytes must be nonnegative");
        long length = input.length();
        input.seek(Math.max(0, length - maxBytes));
        long remaining = Math.min(length, (long) maxBytes);
        byte[] buffer = new byte[8192];
        while (remaining > 0) {
            int count = input.read(buffer, 0, (int) Math.min(buffer.length, remaining));
            if (count == -1) break;
            output.write(buffer, 0, count);
            remaining -= count;
        }
    }
}
