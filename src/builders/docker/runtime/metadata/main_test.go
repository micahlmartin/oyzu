package main

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

var fixtureFacts = selectionFacts{"linux/amd64", "315532800"}

func analyzeFixture(body []byte) (metadata, error) {
	return analyze(body, fixtureFacts)
}

func TestNativeStagesAndHeredocs(t *testing.T) {
	m, err := analyzeFixture([]byte("FROM scratch AS content\nCOPY <<EOF /file\nnot a FROM instruction\nEOF\nFROM content AS copy\nFROM scratch\nCOPY --from=1 /file /file\n"))
	if err != nil {
		t.Fatal(err)
	}
	if len(m.Stages) != 3 || len(m.Requirements) != 0 {
		t.Fatalf("unexpected native metadata: %+v", m)
	}
}

func TestExternalAndSensitiveRequirements(t *testing.T) {
	cases := []struct{ body, kind, reference string }{
		{"FROM registry.example/base:1\n", "image", "registry.example/base:1"},
		{"FROM 0\n", "image", "0"},
		{"FROM self AS self\n", "image", "self"},
		{"FROM --platform=linux/arm64 scratch\n", "platform", "linux/arm64"},
		{"FROM scratch\nCOPY --from=external /app /app\n", "image-or-context", "external"},
		{"FROM scratch\nADD https://example.invalid/archive /app\n", "add-source", "https://example.invalid/archive"},
		{"FROM scratch\nADD ${URL} /app\n", "add-source", "${URL}"},
		{"FROM scratch\nRUN --network=host [\"/app\"]\n", "host-network", ""},
		{"FROM scratch\nRUN --mount=type=secret,id=token [\"/app\"]\n", "secret", ""},
		{"FROM scratch\nRUN --mount=type=ssh [\"/app\"]\n", "ssh", ""},
		{"FROM scratch\nRUN --mount=type=cache,target=/cache [\"/app\"]\n", "cache-mount", ""},
		{"FROM scratch\nRUN --mount=type=bind,from=dependencies,target=/deps [\"/app\"]\n", "image-or-context", "dependencies"},
		{"FROM scratch\nRUN --mount=type=bind,target=${WHERE} [\"/app\"]\n", "dynamic-mount", ""},
		{"FROM scratch\nONBUILD RUN echo later\n", "onbuild", ""},
		{"# syntax=docker/dockerfile:1\nFROM scratch\n", "frontend", "docker/dockerfile:1"},
	}
	for _, c := range cases {
		t.Run(c.kind+"/"+c.reference, func(t *testing.T) {
			m, err := analyzeFixture([]byte(c.body))
			if err != nil {
				t.Fatal(err)
			}
			found := false
			for _, r := range m.Requirements {
				if r.Kind == c.kind && r.Reference == c.reference && r.Line > 0 {
					found = true
				}
			}
			if !found {
				t.Fatalf("missing %s: %+v", c.kind, m.Requirements)
			}
		})
	}
}

func TestNativeParserRejectsUnknownOrMalformedInstructions(t *testing.T) {
	for _, body := range []string{"FROM\n", "FROM scratch\nUNKNOWN anything\n", "FROM scratch\nRUN --mount=type=invalid,target=/tmp true\n", "FROM scratch\nRUN --network=hostile true\n", "# comment only\n"} {
		if _, err := analyzeFixture([]byte(body)); err == nil {
			t.Fatalf("accepted invalid Dockerfile: %s", body)
		}
	}
}

func write(t *testing.T, root, name, body string) {
	t.Helper()
	path := filepath.Join(root, name)
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(body), 0644); err != nil {
		t.Fatal(err)
	}
}

func TestNativeIgnoreNegationsAndDockerfileOverride(t *testing.T) {
	root := t.TempDir()
	write(t, root, "Dockerfile", "FROM scratch\n")
	write(t, root, ".dockerignore", "# native comments\nDockerfile\n**/*.tmp\ncache\n!cache/keep.txt\n")
	for _, name := range []string{"a.tmp", "nested/b.tmp", "nested/file.txt", "cache/keep.txt", "cache/drop.txt"} {
		write(t, root, name, "payload")
	}
	m, err := inspect(root, fixtureFacts)
	if err != nil {
		t.Fatal(err)
	}
	want := []string{".dockerignore", "cache/keep.txt", "nested/file.txt"}
	if !reflect.DeepEqual(m.Context.Files, want) {
		t.Fatalf("ignored context = %v, want %v", m.Context.Files, want)
	}
	write(t, root, "Dockerfile.dockerignore", "*\n!a.tmp\n")
	m, err = inspect(root, fixtureFacts)
	if err != nil {
		t.Fatal(err)
	}
	if m.Context.IgnoreFile != "Dockerfile.dockerignore" || !reflect.DeepEqual(m.Context.Files, []string{"a.tmp"}) {
		t.Fatalf("Dockerfile-specific ignore did not take precedence: %+v", m.Context)
	}
}

func TestBoundedMetadataAndNoLinkTraversal(t *testing.T) {
	root := t.TempDir()
	write(t, root, "Dockerfile", strings.Repeat("#", metadataLimit+1))
	if _, err := inspect(root, fixtureFacts); err == nil {
		t.Fatal("accepted oversized Dockerfile")
	}
	write(t, root, "Dockerfile", "FROM scratch\n")
	write(t, root, "target", "private")
	if err := os.Symlink(filepath.Join(root, "target"), filepath.Join(root, "link")); err != nil {
		t.Skipf("symlinks unavailable: %v", err)
	}
	if _, err := inspect(root, fixtureFacts); err == nil {
		t.Fatal("accepted a symlink in captured context")
	}
}
