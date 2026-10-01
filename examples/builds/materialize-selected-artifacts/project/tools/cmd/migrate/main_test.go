package main

import "testing"

func TestName(t *testing.T) {
 if name() != "migrate" { t.Fatal("wrong command artifact") }
}
