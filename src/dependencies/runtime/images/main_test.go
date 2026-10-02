package main

import (
	"archive/tar"
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/google/go-containerregistry/pkg/name"
	v1 "github.com/google/go-containerregistry/pkg/v1"
	"github.com/google/go-containerregistry/pkg/v1/empty"
	"github.com/google/go-containerregistry/pkg/v1/layout"
	"github.com/google/go-containerregistry/pkg/v1/mutate"
	"github.com/google/go-containerregistry/pkg/v1/tarball"
	"github.com/google/go-containerregistry/pkg/v1/validate"
)

func fixture(t *testing.T, onbuild bool) (string, string) {
	t.Helper()
	var body bytes.Buffer
	writer := tar.NewWriter(&body)
	if err := writer.WriteHeader(&tar.Header{Name: "greeting.txt", Mode: 0644, Size: 6}); err != nil {
		t.Fatal(err)
	}
	if _, err := writer.Write([]byte("hello\n")); err != nil {
		t.Fatal(err)
	}
	if err := writer.Close(); err != nil {
		t.Fatal(err)
	}
	layer, err := tarball.LayerFromReader(bytes.NewReader(body.Bytes()))
	if err != nil {
		t.Fatal(err)
	}
	image, err := mutate.AppendLayers(empty.Image, layer)
	if err != nil {
		t.Fatal(err)
	}
	config, err := image.ConfigFile()
	if err != nil {
		t.Fatal(err)
	}
	config.OS = "linux"
	config.Architecture = "amd64"
	config.Config.User = "65532"
	config.Config.Env = []string{"VISIBLE=public"}
	if onbuild {
		config.Config.OnBuild = []string{"RUN unexpected-command"}
	}
	image, err = mutate.ConfigFile(image, config)
	if err != nil {
		t.Fatal(err)
	}
	digest, err := image.ConfigName()
	if err != nil {
		t.Fatal(err)
	}
	tag, err := name.NewTag("example.invalid/project/base:1")
	if err != nil {
		t.Fatal(err)
	}
	filename := filepath.Join(t.TempDir(), "native image.tar")
	if err := tarball.WriteToFile(filename, tag, image); err != nil {
		t.Fatal(err)
	}
	return filename, digest.String()
}

func TestNativeImageCapturePreservesConfigLayersAndIdentity(t *testing.T) {
	input, expected := fixture(t, false)
	original, err := os.ReadFile(input)
	if err != nil {
		t.Fatal(err)
	}
	var first string
	for _, reference := range []string{"docker.io/library/alpine:latest", "alpine"} {
		output := filepath.Join(t.TempDir(), "captured")
		facts, err := capture(input, output, expected, "linux/amd64", reference)
		if err != nil {
			t.Fatal(err)
		}
		if facts.Config != expected || facts.Name != "alpine" {
			t.Fatalf("wrong binding: %+v", facts)
		}
		if first != "" && first != facts.Manifest {
			t.Fatal("same image captured with different identity")
		}
		first = facts.Manifest
		store, err := layout.FromPath(output)
		if err != nil {
			t.Fatal(err)
		}
		digest, err := v1.NewHash(facts.Manifest)
		if err != nil {
			t.Fatal(err)
		}
		image, err := store.Image(digest)
		if err != nil {
			t.Fatal(err)
		}
		if err := validate.Image(image); err != nil {
			t.Fatal(err)
		}
		config, err := image.ConfigFile()
		if err != nil {
			t.Fatal(err)
		}
		if config.Config.User != "65532" || config.Config.Env[0] != "VISIBLE=public" {
			t.Fatal("runtime config changed")
		}
		layers, err := image.Layers()
		if err != nil || len(layers) != 1 {
			t.Fatal("missing layer", err)
		}
		stream, err := layers[0].Uncompressed()
		if err != nil {
			t.Fatal(err)
		}
		archive := tar.NewReader(stream)
		member, err := archive.Next()
		if err != nil || member.Name != "greeting.txt" {
			t.Fatal("missing layer payload", err)
		}
		stream.Close()
		if _, err := capture(input, output, expected, "linux/amd64", reference); err == nil {
			t.Fatal("overwrote existing store")
		}
	}
	after, err := os.ReadFile(input)
	if err != nil || !bytes.Equal(original, after) {
		t.Fatal("transport input mutated", err)
	}
}

func TestImageAdmissionRejectsMismatchAndHiddenOnbuild(t *testing.T) {
	input, expected := fixture(t, false)
	for _, test := range []struct{ digest, platform string }{
		{"sha256:" + strings.Repeat("0", 64), "linux/amd64"},
		{expected, "linux/arm64"},
	} {
		if _, err := capture(input, filepath.Join(t.TempDir(), "store"), test.digest, test.platform, "base:1"); err == nil {
			t.Fatal("accepted mismatch")
		}
	}
	input, expected = fixture(t, true)
	if _, err := capture(input, filepath.Join(t.TempDir(), "store"), expected, "linux/amd64", "base:1"); err == nil || !strings.Contains(err.Error(), "ONBUILD") {
		t.Fatal("accepted hidden ONBUILD", err)
	}
}

func TestTransportPathsAndLinksAreRejectedWithoutExtraction(t *testing.T) {
	for _, header := range []*tar.Header{
		{Name: "../escape", Mode: 0644}, {Name: "/absolute", Mode: 0644},
		{Name: "link", Typeflag: tar.TypeSymlink, Linkname: "../escape"},
		{Name: "a/../alias", Mode: 0644},
	} {
		filename := filepath.Join(t.TempDir(), "unsafe.tar")
		file, err := os.Create(filename)
		if err != nil {
			t.Fatal(err)
		}
		writer := tar.NewWriter(file)
		if err := writer.WriteHeader(header); err != nil {
			t.Fatal(err)
		}
		writer.Close()
		file.Close()
		if err := inspectArchive(filename); err == nil {
			t.Fatalf("accepted %s", header.Name)
		}
	}
}
