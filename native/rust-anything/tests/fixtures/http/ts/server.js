const http = require("http");

const TEXT = "text/plain; charset=utf-8";
const port = Number(process.env.PORT || 8080);

function send(res, status, type, body) {
  res.writeHead(status, { "Content-Type": type });
  res.end(body);
}

function route(req, res, body) {
  const path = req.url.split("?")[0];
  const parts = path.split("/").slice(1);
  if (req.method === "GET" && path === "/hello") {
    return send(res, 200, "application/json", JSON.stringify({ message: "hello" }));
  }
  if (req.method === "POST" && path === "/upper") {
    return send(res, 200, TEXT, body.toUpperCase());
  }
  if (req.method === "GET" && parts.length === 3 && parts[0] === "div") {
    let a, b;
    try {
      [a, b] = parts.slice(1).map(decodeURIComponent);
    } catch {
      return send(res, 400, TEXT, "error: not a number\n");
    }
    if (!/^-?\d+$/.test(a) || !/^-?\d+$/.test(b)) {
      return send(res, 400, TEXT, "error: not a number\n");
    }
    const [x, y] = [Number(a), Number(b)];
    if (y === 0) {
      return send(res, 400, TEXT, "error: division by zero\n");
    }
    return send(res, 200, TEXT, `${x} / ${y} = ${Math.trunc(x / y)} remainder ${x % y}\n`);
  }
  if (req.method === "GET" && path === "/now") {
    return send(res, 200, TEXT, `now: ${process.hrtime.bigint()}\n`);
  }
  send(res, 404, TEXT, "not found\n");
}

http
  .createServer((req, res) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => route(req, res, Buffer.concat(chunks).toString("utf8")));
  })
  .listen(port, "127.0.0.1");
