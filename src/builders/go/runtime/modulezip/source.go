package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
)

type sourceFile struct {
	path, name string
	data       []byte
}

func (f sourceFile) Path() string { return f.name }
func (f sourceFile) Lstat() (os.FileInfo, error) {
	info, err := os.Lstat(f.path)
	if err == nil && f.data != nil {
		return sizedInfo{info, int64(len(f.data))}, nil
	}
	return info, err
}
func (f sourceFile) Open() (io.ReadCloser, error) {
	if f.data != nil {
		return io.NopCloser(bytes.NewReader(f.data)), nil
	}
	return os.Open(f.path)
}

type sizedInfo struct {
	os.FileInfo
	size int64
}

func (i sizedInfo) Size() int64 { return i.size }

func contained(root, relative string) (string, error) {
	if filepath.IsAbs(relative) {
		return "", fmt.Errorf("absolute module directory")
	}
	path, err := filepath.EvalSymlinks(filepath.Join(root, relative))
	if err != nil {
		return "", err
	}
	rel, err := filepath.Rel(root, path)
	if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("module directory escapes captured source: %s", relative)
	}
	return path, nil
}
