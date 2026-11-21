package com.example.text;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class StringsTest {
    @Test
    void slugify() {
        assertEquals("hello-world", Strings.slugify("Hello, World!", 0));
        assertEquals("a-b", Strings.slugify("a²b", 0));
        assertEquals("a-b", Strings.slugify("aⅠb", 0));
        assertEquals("a-b", Strings.slugify("a𐐀b", 0));
        assertEquals("ab-", Strings.slugify("ab²cd", 3));
    }
}
