require "socket"

TEXT = "text/plain; charset=utf-8"
REASONS = { 200 => "OK", 400 => "Bad Request", 404 => "Not Found" }.freeze

def respond(client, status, type, body)
  body = body.b
  client.write("HTTP/1.1 #{status} #{REASONS[status]}\r\nContent-Type: #{type}\r\n" \
               "Content-Length: #{body.bytesize}\r\nConnection: close\r\n\r\n")
  client.write(body)
end

def unquote(text)
  text.b.gsub(/%([0-9A-Fa-f]{2})/) { Regexp.last_match(1).hex.chr }.force_encoding("UTF-8")
end

def route(method, path, body)
  parts = path.split("/", -1)[1..] || []
  if method == "GET" && path == "/hello"
    [200, "application/json", '{"message":"hello"}']
  elsif method == "POST" && path == "/upper"
    [200, TEXT, body.scrub.upcase]
  elsif method == "GET" && parts.size == 3 && parts[0] == "div"
    a, b = parts[1..].map { |p| unquote(p) }
    return [400, TEXT, "error: not a number\n"] unless [a, b].all? { |x| x.b.match?(/\A-?\d+\z/) }

    a, b = a.to_i, b.to_i
    return [400, TEXT, "error: division by zero\n"] if b.zero?

    [200, TEXT, "#{a} / #{b} = #{a / b} remainder #{a % b}\n"]
  elsif method == "GET" && path == "/now"
    [200, TEXT, "now: #{Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond)}\n"]
  else
    [404, TEXT, "not found\n"]
  end
end

server = TCPServer.new("127.0.0.1", Integer(ENV.fetch("PORT", "8080")))
loop do
  client = server.accept
  begin
    request_line = client.gets
    next unless request_line

    method, target = request_line.split(" ")
    headers = {}
    while (line = client.gets) && line != "\r\n"
      key, val = line.split(":", 2)
      headers[key.strip.downcase] = val.to_s.strip
    end
    body = client.read(headers.fetch("content-length", "0").to_i).to_s.force_encoding("UTF-8")
    respond(client, *route(method, target.split("?", 2)[0], body))
  ensure
    client.close
  end
end
