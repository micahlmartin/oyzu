package main

import (
	"fmt"
	"runtime"
)

func identity() string { return runtime.GOOS + "/" + runtime.GOARCH }
func main()            { fmt.Println(identity()) }
