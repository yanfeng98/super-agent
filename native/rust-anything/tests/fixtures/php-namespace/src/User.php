<?php

namespace Acme;

use Monolog\Logger;
use Stripe\StripeClient;
use Acme\Greeter;
use function Acme\helper;

class User
{
    public function hello(string $name): string
    {
        (new Logger('app'))->info('hello');
        new StripeClient('sk_test');
        return helper((new Greeter())->greet($name));
    }
}
