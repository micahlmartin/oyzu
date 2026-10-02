package main

import (
	"fmt"
	"os"
	"path/filepath"
)

func main() {
	if len(os.Args) != 5 {
		fmt.Fprintln(os.Stderr, "usage: oyzu-go-modulezip METADATA SNAPSHOT_VERSION BUILDER OUTPUT")
		os.Exit(2)
	}
	root, err := os.Getwd()
	if err == nil {
		root, err = filepath.EvalSymlinks(root)
	}
	if err == nil {
		err = archive(root, os.Args[1], os.Args[2], os.Args[3], os.Args[4])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
