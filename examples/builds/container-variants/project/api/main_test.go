package main

import (
 "runtime"
 "testing"
)

func TestIdentity(t *testing.T) {
 if identity() != runtime.GOOS+"/"+runtime.GOARCH { t.Fatal("incorrect platform identity") }
}
