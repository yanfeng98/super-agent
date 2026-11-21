<?php

declare(strict_types=1);

use Acme\Textkit\Text\Slug;
use PHPUnit\Framework\TestCase;

final class SlugTest extends TestCase
{
    public function testSlugify(): void
    {
        $this->assertSame('grusse-aus-koln', Slug::slugify('Grüße aus Köln'));
    }
}
