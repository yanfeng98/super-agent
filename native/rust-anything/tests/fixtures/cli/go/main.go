package main

import (
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"
	"time"
)

func usage() int {
	fmt.Fprintln(os.Stderr, "usage: tcli upper|len|div|words|copy|now")
	return 1
}

func run(argv []string) int {
	if len(argv) == 0 {
		return usage()
	}
	cmd, args := argv[0], argv[1:]
	switch {
	case cmd == "upper":
		fmt.Println(strings.ToUpper(strings.Join(args, " ")))
	case cmd == "len" && len(args) == 1:
		fmt.Println(len(args[0]))
	case cmd == "div" && len(args) == 2:
		a, errA := strconv.ParseInt(args[0], 10, 64)
		b, errB := strconv.ParseInt(args[1], 10, 64)
		if errA != nil || errB != nil {
			fmt.Fprintln(os.Stderr, "error: not a number")
			return 2
		}
		if b == 0 {
			fmt.Fprintln(os.Stderr, "error: division by zero")
			return 2
		}
		fmt.Printf("%d / %d = %d remainder %d\n", a, b, a/b, a%b)
	case cmd == "words":
		data, _ := io.ReadAll(os.Stdin)
		fmt.Println(len(strings.Fields(string(data))))
	case cmd == "copy" && len(args) == 2:
		data, err := os.ReadFile(args[0])
		if err != nil {
			fmt.Fprintf(os.Stderr, "error: cannot read %s\n", args[0])
			return 1
		}
		if err := os.WriteFile(args[1], []byte(strings.ToUpper(string(data))), 0o644); err != nil {
			fmt.Fprintf(os.Stderr, "error: cannot write %s\n", args[1])
			return 1
		}
		fmt.Printf("copied %d bytes\n", len(data))
	case cmd == "now":
		fmt.Printf("now: %d\n", time.Now().UnixNano())
	default:
		return usage()
	}
	return 0
}

func main() {
	os.Exit(run(os.Args[1:]))
}
