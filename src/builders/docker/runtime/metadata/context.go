package main

import (
	"bytes"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"

	"github.com/moby/patternmatcher"
	"github.com/moby/patternmatcher/ignorefile"
)

type contextFiles struct {
	IgnoreFile string   `json:"ignoreFile,omitempty"`
	Files      []string `json:"files"`
}

// Enumerate captured files using Docker's matcher. Do not prune ignored
// directories: a later negation may restore one of their descendants.
func inspectContext(root string) (contextFiles, error) {
	result := contextFiles{Files: []string{}}
	var patterns []string
	for _, name := range []string{"Dockerfile.dockerignore", ".dockerignore"} {
		body, err := readBounded(filepath.Join(root, name))
		if os.IsNotExist(err) {
			continue
		}
		if err != nil {
			return result, err
		}
		patterns, err = ignorefile.ReadAll(bytes.NewReader(body))
		if err != nil {
			return result, err
		}
		result.IgnoreFile = name
		break
	}
	matcher, err := patternmatcher.New(patterns)
	if err != nil {
		return result, err
	}
	count := 0
	err = filepath.WalkDir(root, func(path string, entry fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		count++
		if count > 100000 {
			return fmt.Errorf("context exceeds 100000 entries")
		}
		if entry.Type()&os.ModeSymlink != 0 {
			return fmt.Errorf("context contains a symlink: %s", path)
		}
		if entry.IsDir() {
			return nil
		}
		if !entry.Type().IsRegular() {
			return fmt.Errorf("context contains a nonregular file: %s", path)
		}
		relative, err := filepath.Rel(root, path)
		if err != nil {
			return err
		}
		ignored, err := matcher.MatchesOrParentMatches(relative)
		if err != nil {
			return err
		}
		if !ignored {
			result.Files = append(result.Files, filepath.ToSlash(relative))
		}
		return nil
	})
	return result, err
}
