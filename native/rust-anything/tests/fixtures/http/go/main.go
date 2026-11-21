package main

import (
	"fmt"
	"io"
	"net/http"
	"os"
	"strconv"
	"strings"
	"time"
)

const text = "text/plain; charset=utf-8"

func send(w http.ResponseWriter, status int, ctype, body string) {
	w.Header().Set("Content-Type", ctype)
	w.WriteHeader(status)
	io.WriteString(w, body)
}

func handle(w http.ResponseWriter, r *http.Request) {
	body, _ := io.ReadAll(r.Body)
	path := r.URL.Path
	parts := strings.Split(path, "/")[1:]
	switch {
	case r.Method == "GET" && path == "/hello":
		send(w, 200, "application/json", `{"message":"hello"}`)
	case r.Method == "POST" && path == "/upper":
		send(w, 200, text, strings.ToUpper(string(body)))
	case r.Method == "GET" && len(parts) == 3 && parts[0] == "div":
		a, errA := strconv.ParseInt(parts[1], 10, 64)
		b, errB := strconv.ParseInt(parts[2], 10, 64)
		if errA != nil || errB != nil {
			send(w, 400, text, "error: not a number\n")
			return
		}
		if b == 0 {
			send(w, 400, text, "error: division by zero\n")
			return
		}
		send(w, 200, text, fmt.Sprintf("%d / %d = %d remainder %d\n", a, b, a/b, a%b))
	case r.Method == "GET" && path == "/now":
		send(w, 200, text, fmt.Sprintf("now: %d\n", time.Now().UnixNano()))
	default:
		send(w, 404, text, "not found\n")
	}
}

func main() {
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	if err := http.ListenAndServe("127.0.0.1:"+port, http.HandlerFunc(handle)); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
