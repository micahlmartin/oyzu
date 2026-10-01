// Native Dockerfile metadata only. The Rust builder owns admission and planning.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/moby/buildkit/frontend/dockerfile/instructions"
	"github.com/moby/buildkit/frontend/dockerfile/parser"
)

const metadataLimit = 1024 * 1024

type requirement struct {
	Kind      string `json:"kind"`
	Reference string `json:"reference,omitempty"`
	Stage     int    `json:"stage"`
	Line      int    `json:"line"`
}

type stage struct {
	Name     string `json:"name"`
	Base     string `json:"base"`
	Platform string `json:"platform,omitempty"`
}

type metadata struct {
	SchemaVersion string        `json:"schemaVersion"`
	Frontend      string        `json:"frontend"`
	Stages        []stage       `json:"stages"`
	Requirements  []requirement `json:"requirements"`
	Context       contextFiles  `json:"context"`
}

func readBounded(path string) ([]byte, error) {
	info, err := os.Lstat(path)
	if err != nil {
		return nil, err
	}
	if !info.Mode().IsRegular() {
		return nil, fmt.Errorf("metadata input is not a regular file: %s", path)
	}
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	body, err := io.ReadAll(io.LimitReader(f, metadataLimit+1))
	if len(body) > metadataLimit {
		return nil, fmt.Errorf("metadata input exceeds 1 MiB")
	}
	return body, err
}

func analyze(body []byte) (metadata, error) {
	m := metadata{SchemaVersion: "v1alpha1", Frontend: "dockerfile.v0", Stages: []stage{}, Requirements: []requirement{}}
	if frontend, _, _, ok := parser.DetectSyntax(body); ok {
		m.Frontend = frontend
		m.Requirements = append(m.Requirements, requirement{Kind: "frontend", Reference: frontend, Stage: -1, Line: 1})
	}
	parsed, err := parser.Parse(bytes.NewReader(body))
	if err != nil {
		return m, err
	}
	stages, _, err := instructions.Parse(parsed.AST, nil)
	if err != nil {
		return m, err
	}
	if len(stages) == 0 {
		return m, fmt.Errorf("Dockerfile has no stages")
	}
	// Native aliases can refer forward. We retain their identities; BuildKit
	// remains responsible for native stage-cycle validation during conversion.
	aliases := map[string]bool{}
	for _, s := range stages {
		if s.Name != "" {
			aliases[strings.ToLower(s.Name)] = true
		}
	}
	internal := func(value string) bool {
		if aliases[strings.ToLower(value)] {
			return true
		}
		n, err := strconv.Atoi(value)
		return err == nil && n >= 0 && n < len(stages)
	}
	previous := map[string]bool{}
	for index, s := range stages {
		m.Stages = append(m.Stages, stage{Name: s.Name, Base: s.BaseName, Platform: s.Platform})
		add := func(kind, reference string, line int) {
			m.Requirements = append(m.Requirements, requirement{Kind: kind, Reference: reference, Stage: index, Line: line})
		}
		line := s.Location[0].Start.Line
		if strings.Contains(s.BaseName, "$") {
			add("dynamic-base", s.BaseName, line)
		} else if s.BaseName != "scratch" && !previous[strings.ToLower(s.BaseName)] {
			add("image", s.BaseName, line)
		}
		if s.Name != "" {
			previous[strings.ToLower(s.Name)] = true
		}
		if s.Platform != "" {
			add("platform", s.Platform, line)
		}
		for _, command := range s.Commands {
			line := command.Location()[0].Start.Line
			switch c := command.(type) {
			case *instructions.CopyCommand:
				if c.From != "" && !internal(c.From) {
					add("image-or-context", c.From, line)
				}
			case *instructions.AddCommand:
				// ADD supports local archives, remote HTTP and Git sources. Keep
				// all sources explicit; admission must classify/capture them.
				for _, source := range c.SourcePaths {
					add("add-source", source, line)
				}
			case *instructions.OnbuildCommand:
				add("onbuild", "", line)
			case *instructions.RunCommand:
				if instructions.GetNetwork(c) == instructions.NetworkHost {
					add("host-network", "", line)
				}
				// Parsing defers mount fields until native expansion. Identity
				// expansion exposes literal mounts without evaluating RUN code;
				// variable mounts need a later, captured environment resolution.
				dynamic := false
				if err := c.Expand(func(value string) (string, error) {
					if strings.Contains(value, "$") {
						dynamic = true
					}
					return value, nil
				}); err != nil {
					return m, err
				}
				if dynamic {
					add("dynamic-mount", "", line)
				}
				for _, mount := range instructions.GetMounts(c) {
					switch mount.Type {
					case instructions.MountTypeSecret:
						add("secret", "", line)
					case instructions.MountTypeSSH:
						add("ssh", "", line)
					case instructions.MountTypeCache:
						add("cache-mount", "", line)
					}
					if mount.From != "" && !internal(mount.From) {
						add("image-or-context", mount.From, line)
					}
				}
			}
		}
	}
	return m, nil
}

func inspect(root string) (metadata, error) {
	body, err := readBounded(filepath.Join(root, "Dockerfile"))
	if err != nil {
		return metadata{}, err
	}
	m, err := analyze(body)
	if err != nil {
		return m, err
	}
	m.Context, err = inspectContext(root)
	return m, err
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: oyzu-docker-metadata <captured-context>")
		os.Exit(2)
	}
	m, err := inspect(os.Args[1])
	if err == nil {
		err = json.NewEncoder(os.Stdout).Encode(m)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
