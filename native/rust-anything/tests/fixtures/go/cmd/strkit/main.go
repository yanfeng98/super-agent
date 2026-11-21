package main

import (
	"fmt"
	"os"

	"example.com/strkit/strkit"
)

func main() {
	for _, arg := range os.Args[1:] {
		fmt.Println(strkit.Slugify(arg, 0), strkit.WordFreq(arg))
	}
}
