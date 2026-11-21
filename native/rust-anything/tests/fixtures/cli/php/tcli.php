<?php

function usage(): int
{
    fwrite(STDERR, "usage: tcli upper|len|div|words|copy|now\n");
    return 1;
}

function run(array $argv): int
{
    if (count($argv) === 0) {
        return usage();
    }
    $cmd = array_shift($argv);
    $args = $argv;
    if ($cmd === 'upper') {
        echo mb_strtoupper(implode(' ', $args), 'UTF-8'), "\n";
    } elseif ($cmd === 'len' && count($args) === 1) {
        echo mb_strlen($args[0], 'UTF-8'), "\n";
    } elseif ($cmd === 'div' && count($args) === 2) {
        foreach ($args as $a) {
            if (!preg_match('/\A-?\d+\z/', $a)) {
                fwrite(STDERR, "error: not a number\n");
                return 2;
            }
        }
        [$a, $b] = array_map('intval', $args);
        if ($b === 0) {
            fwrite(STDERR, "error: division by zero\n");
            return 2;
        }
        echo "$a / $b = ", intdiv($a, $b), " remainder ", $a % $b, "\n";
    } elseif ($cmd === 'words') {
        $text = trim(stream_get_contents(STDIN), " \t\n\r\v\f");
        echo $text === '' ? 0 : count(preg_split('/[ \t\n\r\v\f]+/', $text)), "\n";
    } elseif ($cmd === 'copy' && count($args) === 2) {
        $text = $args[0] === '' ? false : @file_get_contents($args[0]);
        if ($text === false) {
            fwrite(STDERR, "error: cannot read {$args[0]}\n");
            return 1;
        }
        if ($args[1] === '' || @file_put_contents($args[1], mb_strtoupper($text, 'UTF-8')) === false) {
            fwrite(STDERR, "error: cannot write {$args[1]}\n");
            return 1;
        }
        echo 'copied ', mb_strlen($text, 'UTF-8'), " characters\n";
    } elseif ($cmd === 'now') {
        echo 'now: ', hrtime(true), "\n";
    } else {
        return usage();
    }
    return 0;
}

exit(run(array_slice($argv, 1)));
