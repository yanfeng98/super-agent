<?php

declare(strict_types=1);

namespace Acme\Textkit\Util;

final class Ascii
{
    public static function fold(string $text): string
    {
        return strtr($text, ['ä' => 'a', 'ö' => 'o', 'ü' => 'u', 'ß' => 'ss', 'é' => 'e']);
    }
}
