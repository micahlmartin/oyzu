package main

import "testing"

func TestIdentifier(t *testing.T) {
	if identifier() != "00000000-0000-0000-0000-000000000000" {
		t.Fatal("unexpected UUID")
	}
}
