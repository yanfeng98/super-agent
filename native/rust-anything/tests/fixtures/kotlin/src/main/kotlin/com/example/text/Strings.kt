package com.example.text

/** Lowercases text and joins its letter-or-digit runs with '-', cut to max chars (max <= 0: no limit). */
fun slugify(text: String, max: Int): String {
    val out = StringBuilder()
    var dash = false
    for (c in text.lowercase()) {
        if (c.isLetterOrDigit()) {
            out.append(c)
            dash = false
        } else if (!dash && out.isNotEmpty()) {
            out.append('-')
            dash = true
        }
    }
    var slug = out.toString()
    if (slug.endsWith("-")) {
        slug = slug.dropLast(1)
    }
    if (max > 0 && slug.length > max) {
        slug = slug.substring(0, max)
    }
    return slug
}

/** The whitespace-separated words of text. */
fun words(text: String): List<String> =
    text.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }

/** Word counts, in order of first appearance. */
fun frequencies(text: String): Map<String, Int> {
    val counts = LinkedHashMap<String, Int>()
    for (w in words(text)) {
        val key = w.lowercase()
        counts[key] = (counts[key] ?: 0) + 1
    }
    return counts
}

object Nums {
    /** Greatest common divisor, absolute values. */
    @JvmStatic
    fun gcd(a: Long, b: Long): Long {
        var x = if (a < 0) -a else a
        var y = if (b < 0) -b else b
        while (y != 0L) {
            val t = x % y
            x = y
            y = t
        }
        return x
    }

    /** Pages needed for total items at perPage per page. */
    @JvmStatic
    fun pages(total: Int, perPage: Int): Int = (total + perPage - 1) / perPage
}
