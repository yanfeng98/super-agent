import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetSocketAddress;
import java.nio.charset.StandardCharsets;

public class Server {
    static final String TEXT = "text/plain; charset=utf-8";

    static void send(HttpExchange ex, int status, String type, String body) throws IOException {
        byte[] data = body.getBytes(StandardCharsets.UTF_8);
        ex.getResponseHeaders().set("Content-Type", type);
        ex.sendResponseHeaders(status, data.length == 0 ? -1 : data.length);
        try (OutputStream out = ex.getResponseBody()) {
            out.write(data);
        }
    }

    static void route(HttpExchange ex) throws IOException {
        String method = ex.getRequestMethod();
        String path = ex.getRequestURI().getPath();
        String body = new String(ex.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
        String[] parts = path.substring(1).split("/", -1);
        if (method.equals("GET") && path.equals("/hello")) {
            send(ex, 200, "application/json", "{\"message\":\"hello\"}");
        } else if (method.equals("POST") && path.equals("/upper")) {
            send(ex, 200, TEXT, body.toUpperCase(java.util.Locale.ROOT));
        } else if (method.equals("GET") && parts.length == 3 && parts[0].equals("div")) {
            long a, b;
            try {
                a = Long.parseLong(parts[1]);
                b = Long.parseLong(parts[2]);
            } catch (NumberFormatException e) {
                send(ex, 400, TEXT, "error: not a number\n");
                return;
            }
            if (b == 0) {
                send(ex, 400, TEXT, "error: division by zero\n");
                return;
            }
            send(ex, 200, TEXT, a + " / " + b + " = " + (a / b) + " remainder " + (a % b) + "\n");
        } else if (method.equals("GET") && path.equals("/now")) {
            send(ex, 200, TEXT, "now: " + System.nanoTime() + "\n");
        } else {
            send(ex, 404, TEXT, "not found\n");
        }
    }

    public static void main(String[] args) throws IOException {
        int port = Integer.parseInt(System.getenv().getOrDefault("PORT", "8080"));
        HttpServer server = HttpServer.create(new InetSocketAddress("127.0.0.1", port), 0);
        server.createContext("/", ex -> {
            try {
                route(ex);
            } catch (Exception e) {
                send(ex, 500, TEXT, "error\n");
            }
        });
        server.start();
    }
}
