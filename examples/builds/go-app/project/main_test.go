package main

import "testing"

func TestGreeting(t *testing.T) {
	if got := greeting("Oyzu"); got != "Hello, Oyzu!" {
		t.Fatalf("unexpected greeting: %q", got)
	}
}
