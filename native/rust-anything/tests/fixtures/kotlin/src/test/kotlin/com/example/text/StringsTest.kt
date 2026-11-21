package com.example.text

import org.junit.Assert.assertEquals
import org.junit.Test

class StringsTest {
    @Test
    fun slugifies() {
        assertEquals("hello-world", slugify("Hello, World!", 0))
    }

    @Test
    fun countsWords() {
        assertEquals(mapOf("a" to 2, "b" to 1), frequencies("a b a"))
    }
}
