package com.example.text;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

/** Small string helpers. */
public final class Strings {
    private Strings() {}

    /** Lowercases text and joins its letter-or-digit runs with '-', cut to max chars (max <= 0: no limit). */
    public static String slugify(String text, int max) {
        StringBuilder out = new StringBuilder();
        boolean dash = false;
        for (char c : text.toLowerCase(Locale.ROOT).toCharArray()) {
            if (Character.isLetterOrDigit(c)) {
                out.append(c);
                dash = false;
            } else if (!dash && out.length() > 0) {
                out.append('-');
                dash = true;
            }
        }
        String slug = out.toString();
        if (slug.endsWith("-")) {
            slug = slug.substring(0, slug.length() - 1);
        }
        if (max > 0 && slug.length() > max) {
            slug = slug.substring(0, max);
        }
        return slug;
    }

    /** The whitespace-separated words of text. */
    public static List<String> words(String text) {
        List<String> out = new ArrayList<>();
        for (String w : text.trim().split("\\s+")) {
            if (!w.isEmpty()) {
                out.add(w);
            }
        }
        return out;
    }

    /** Word counts, in order of first appearance. */
    public static Map<String, Integer> frequencies(String text) {
        Map<String, Integer> counts = new LinkedHashMap<>();
        for (String w : words(text)) {
            counts.merge(w.toLowerCase(Locale.ROOT), 1, Integer::sum);
        }
        return counts;
    }

    /** Parses a TCP port. */
    public static int parsePort(String text) {
        int port = Integer.parseInt(text.trim());
        if (port < 1 || port > 65535) {
            throw new IllegalArgumentException("port out of range: " + port);
        }
        return port;
    }

    /** A 31-based rolling hash, like String.hashCode. */
    public static int hash(String text) {
        int h = 0;
        for (int i = 0; i < text.length(); i++) {
            h = 31 * h + text.charAt(i);
        }
        return h;
    }
}
