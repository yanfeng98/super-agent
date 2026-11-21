<?php
// A router for PHP's built-in server: php -S 127.0.0.1:$PORT server.php

const TEXT = 'text/plain; charset=utf-8';

function respond(int $status, string $type, string $body): void
{
    header_remove('X-Powered-By');
    http_response_code($status);
    header("Content-Type: $type");
    echo $body;
}

$method = $_SERVER['REQUEST_METHOD'];
$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH);
$parts = array_slice(explode('/', $path), 1);

if ($method === 'GET' && $path === '/hello') {
    respond(200, 'application/json', '{"message":"hello"}');
} elseif ($method === 'POST' && $path === '/upper') {
    $body = mb_scrub(file_get_contents('php://input'), 'UTF-8');
    respond(200, TEXT, mb_strtoupper($body, 'UTF-8'));
} elseif ($method === 'GET' && count($parts) === 3 && $parts[0] === 'div') {
    [$a, $b] = [rawurldecode($parts[1]), rawurldecode($parts[2])];
    if (!preg_match('/\A-?\d+\z/', $a) || !preg_match('/\A-?\d+\z/', $b)) {
        respond(400, TEXT, "error: not a number\n");
    } elseif ((int) $b === 0) {
        respond(400, TEXT, "error: division by zero\n");
    } else {
        [$a, $b] = [(int) $a, (int) $b];
        respond(200, TEXT, "$a / $b = " . intdiv($a, $b) . " remainder " . ($a % $b) . "\n");
    }
} elseif ($method === 'GET' && $path === '/now') {
    respond(200, TEXT, 'now: ' . hrtime(true) . "\n");
} else {
    respond(404, TEXT, "not found\n");
}
