package math

import "testing"

func TestDouble(t *testing.T) {
	if got := Double(3); got != 6 {
		t.Fatalf("Double(3) = %d", got)
	}
}

func TestNegative(t *testing.T) {
	if got := Double(-1); got != 0 {
		t.Fatalf("Double(-1) = %d", got)
	}
}
