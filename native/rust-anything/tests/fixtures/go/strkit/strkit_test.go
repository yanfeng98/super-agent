package strkit

import "testing"

func TestSlugify(t *testing.T) {
	if got := Slugify("Hello, World!", 0); got != "hello-world" {
		t.Fatalf("got %q", got)
	}
}
