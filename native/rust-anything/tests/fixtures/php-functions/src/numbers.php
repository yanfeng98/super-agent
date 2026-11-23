<?php

namespace Acme\Numbers;

function clamp(int $value, int $lo = 0, int $hi = 10): int
{
    if ($lo > $hi) {
        throw new \InvalidArgumentException('lo is above hi');
    }
    return max($lo, min($hi, $value));
}

/**
 * @param list<float> $xs
 */
function mean(array $xs): float
{
    if ($xs === []) {
        throw new \DomainException('mean of nothing');
    }
    return array_sum($xs) / count($xs);
}

function ratio(float $a, float $b): float
{
    return $a / $b;
}
