module example.com/app

go 1.22

require (
	// pinned until the v2 migration lands
	github.com/spf13/cobra v1.8.0
	github.com/google/uuid v1.6.0 // a trailing comment
	golang.org/x/sys v0.20.0 // indirect
	// github.com/old/thing v0.1.0
)

require github.com/mattn/go-sqlite3 v1.14.22
