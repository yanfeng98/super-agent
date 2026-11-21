// Package strkit holds small string and number helpers.
package strkit

import (
	"errors"
	"strconv"
	"strings"
	"unicode"
)

// Point is a point in the plane.
type Point struct {
	X float64 `json:"x"`
	Y float64 `json:"y"`
}

// Slugify lowercases s, joins its words with '-', and cuts the result to max bytes (max <= 0: no limit).
func Slugify(s string, max int) string {
	var b strings.Builder
	dash := false
	for _, r := range strings.ToLower(s) {
		if unicode.IsLetter(r) || unicode.IsDigit(r) {
			b.WriteRune(r)
			dash = false
		} else if !dash && b.Len() > 0 {
			b.WriteByte('-')
			dash = true
		}
	}
	out := strings.TrimSuffix(b.String(), "-")
	if max > 0 && len(out) > max {
		out = strings.TrimSuffix(out[:max], "-")
	}
	return out
}

// WordFreq counts the whitespace-separated words of text, case-insensitively.
func WordFreq(text string) map[string]int {
	counts := map[string]int{}
	for _, w := range strings.Fields(text) {
		counts[strings.ToLower(w)]++
	}
	return counts
}

// ParsePort parses a TCP port number.
func ParsePort(s string) (int, error) {
	n, err := strconv.Atoi(strings.TrimSpace(s))
	if err != nil {
		return 0, errors.New("not a number")
	}
	if n < 1 || n > 65535 {
		return 0, errors.New("out of range")
	}
	return n, nil
}

// Mean is the average of xs; 0 for none.
func Mean(xs []float64) float64 {
	if len(xs) == 0 {
		return 0
	}
	sum := 0.0
	for _, x := range xs {
		sum += x
	}
	return sum / float64(len(xs))
}

// Midpoint is the point halfway between a and b.
func Midpoint(a, b Point) Point {
	return Point{X: (a.X + b.X) / 2, Y: (a.Y + b.Y) / 2}
}

// Initials returns the first letter of each word, upper-cased.
func Initials(name string) string {
	var out []rune
	for _, w := range strings.Fields(name) {
		out = append(out, unicode.ToUpper([]rune(w)[0]))
	}
	return string(out)
}

// SplitPair splits s at the first sep.
func SplitPair(s, sep string) (string, string, bool) {
	return strings.Cut(s, sep)
}

// Join joins parts with sep.
func Join(sep string, parts ...string) string {
	return strings.Join(parts, sep)
}

// Norm is the distance from the origin.
func (p Point) Norm() float64 {
	return p.X*p.X + p.Y*p.Y
}
