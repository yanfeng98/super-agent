<?php

declare(strict_types=1);

namespace Acme\Textkit\Text;

use Acme\Textkit\Util\Ascii;

final class Slug
{
    /**
     * @param string $text
     * @return string
     */
    public static function slugify(string $text, string $sep = '-'): string
    {
        $ascii = Ascii::fold($text);
        $slug = preg_replace('/[^a-z0-9]+/', $sep, strtolower($ascii));
        return trim($slug, $sep);
    }

    public static function truncate(string $text, int $max): string
    {
        if ($max < 0) {
            throw new \InvalidArgumentException('max must not be negative');
        }
        return mb_substr($text, 0, $max);
    }
}
