package dev.xcaptain.muguet;

import org.json.JSONException;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** Rejects lenient org.json extensions before org.json assembles the real report. */
final class JsonSyntax {
    private static final Pattern NUMBER = Pattern.compile("-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?(?:[eE][+-]?[0-9]+)?");
    private final String text;
    private int position;

    private JsonSyntax(String text) { this.text = text; }

    static void requireJson(String text) throws JSONException {
        JsonSyntax syntax = new JsonSyntax(text);
        syntax.value(0);
        syntax.space();
        if (syntax.position != text.length()) syntax.invalid();
    }

    private void value(int depth) throws JSONException {
        if (depth > 128) invalid();
        space();
        char next = peek();
        if (next == '"') {
            string();
        } else if (next == '{' || next == '[') {
            position++;
            char end = next == '{' ? '}' : ']';
            space();
            if (take(end)) return;
            do {
                space();
                if (next == '{') {
                    string();
                    space();
                    if (!take(':')) invalid();
                }
                value(depth + 1);
                space();
                if (take(end)) return;
                if (!take(',')) invalid();
            } while (true);
        } else if (text.startsWith("true", position)) {
            position += 4;
        } else if (text.startsWith("false", position)) {
            position += 5;
        } else if (text.startsWith("null", position)) {
            position += 4;
        } else {
            Matcher number = NUMBER.matcher(text).region(position, text.length());
            if (!number.lookingAt()) invalid();
            position = number.end();
        }
    }

    private void string() throws JSONException {
        if (!take('"')) invalid();
        while (position < text.length()) {
            char character = text.charAt(position++);
            if (character == '"') return;
            if (character < 0x20) invalid();
            if (character == '\\') {
                if (position == text.length()) invalid();
                char escape = text.charAt(position++);
                if (escape == 'u') {
                    for (int i = 0; i < 4; i++) {
                        char hex = peek();
                        if (!(hex >= '0' && hex <= '9') && !(hex >= 'a' && hex <= 'f')
                                && !(hex >= 'A' && hex <= 'F')) invalid();
                        position++;
                    }
                } else if ("\"\\/bfnrt".indexOf(escape) < 0) invalid();
            }
        }
        invalid();
    }

    private char peek() { return position < text.length() ? text.charAt(position) : 0; }
    private boolean take(char expected) {
        if (peek() != expected) return false;
        position++;
        return true;
    }
    private void space() {
        while (position < text.length() && " \t\r\n".indexOf(text.charAt(position)) >= 0) position++;
    }
    private void invalid() throws JSONException { throw new JSONException("Invalid JSON syntax at offset " + position); }
}
