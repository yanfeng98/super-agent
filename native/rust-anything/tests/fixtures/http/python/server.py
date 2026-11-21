import os
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from urllib.parse import unquote, urlsplit

TEXT = "text/plain; charset=utf-8"


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send(self, status, ctype, body):
        data = body.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def route(self, body):
        path = urlsplit(self.path).path
        parts = path.split("/")[1:]
        if self.command == "GET" and path == "/hello":
            return self.send(200, "application/json", '{"message":"hello"}')
        if self.command == "POST" and path == "/upper":
            return self.send(200, TEXT, body.upper())
        if self.command == "GET" and len(parts) == 3 and parts[0] == "div":
            try:
                a, b = int(unquote(parts[1])), int(unquote(parts[2]))
            except ValueError:
                return self.send(400, TEXT, "error: not a number\n")
            if b == 0:
                return self.send(400, TEXT, "error: division by zero\n")
            return self.send(200, TEXT, f"{a} / {b} = {a // b} remainder {a % b}\n")
        if self.command == "GET" and path == "/now":
            return self.send(200, TEXT, f"now: {time.time_ns()}\n")
        self.send(404, TEXT, "not found\n")

    def do_GET(self):
        self.route("")

    def do_POST(self):
        length = int(self.headers.get("Content-Length") or 0)
        self.route(self.rfile.read(length).decode("utf-8", "replace"))


HTTPServer(("127.0.0.1", int(os.environ.get("PORT", "8080"))), Handler).serve_forever()
