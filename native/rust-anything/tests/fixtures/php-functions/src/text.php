<?php

declare(strict_types=1);

namespace Acme\Text;

function slugify(string $text, string $sep = '-', ?int $max = null): string
{
    $slug = trim(preg_replace('/[^a-z0-9]+/', $sep, strtolower($text)), $sep);
    return $max === null ? $slug : substr($slug, 0, $max);
}

/**
 * @return list<string>
 */
function words(string $text): array
{
    return array_values(array_filter(explode(' ', $text), fn ($w) => $w !== ''));
}

/**
 * @param array<string, int> $counts
 * @return array<string, int>
 */
function top(array $counts, int $n): array
{
    if ($n < 0) {
        throw new \InvalidArgumentException('n must not be negative');
    }
    arsort($counts);
    return array_slice($counts, 0, $n, true);
}

function each_word(string $text): \Generator
{
    foreach (explode(' ', $text) as $word) {
        yield $word;
    }
}

function stamp(\DateTimeInterface $at): string
{
    return $at->format('c');
}

/**
 * @return array<string, int>
 */
function count_words(string $text): array
{
    $counts = [];
    foreach (words($text) as $word) {
        $counts[$word] = ($counts[$word] ?? 0) + 1;
    }
    return $counts;
}
