package main

import "testing"

func TestName(t *testing.T) {
 if name() != "server" { t.Fatal("wrong command artifact") }
}
