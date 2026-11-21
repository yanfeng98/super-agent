import sys
import time


def usage():
    print("usage: tcli upper|len|div|words|copy|now", file=sys.stderr)
    return 1


def main(argv):
    if not argv:
        return usage()
    cmd, args = argv[0], argv[1:]
    if cmd == "upper":
        print(" ".join(args).upper())
    elif cmd == "len" and len(args) == 1:
        print(len(args[0]))
    elif cmd == "div" and len(args) == 2:
        try:
            a, b = int(args[0]), int(args[1])
        except ValueError:
            print("error: not a number", file=sys.stderr)
            return 2
        if b == 0:
            print("error: division by zero", file=sys.stderr)
            return 2
        print(f"{a} / {b} = {a // b} remainder {a % b}")
    elif cmd == "words":
        print(len(sys.stdin.read().split()))
    elif cmd == "copy" and len(args) == 2:
        try:
            with open(args[0], encoding="utf-8") as f:
                text = f.read()
        except OSError:
            print(f"error: cannot read {args[0]}", file=sys.stderr)
            return 1
        try:
            with open(args[1], "w", encoding="utf-8") as f:
                f.write(text.upper())
        except OSError:
            print(f"error: cannot write {args[1]}", file=sys.stderr)
            return 1
        print(f"copied {len(text)} characters")
    elif cmd == "now":
        print(f"now: {time.time_ns()}")
    else:
        return usage()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
