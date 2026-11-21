# tserve

A small C# HTTP server used to test `rust-anything app record --http` and the HTTP front door.
It listens on `PORT` (default 8080).

```console
$ curl http://localhost:8080/hello
{"message":"hello"}
$ curl -X POST -d 'hello world' http://localhost:8080/upper
HELLO WORLD
$ curl http://localhost:8080/div/7/2
$ curl http://localhost:8080/div/1/0
$ curl http://localhost:8080/now
$ curl http://localhost:8080/missing
```
