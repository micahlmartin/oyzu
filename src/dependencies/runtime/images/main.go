// Capture a provisioned image archive without registry, daemon or project execution.
package main

import (
	"archive/tar"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"strings"

	"github.com/distribution/reference"

	v1 "github.com/google/go-containerregistry/pkg/v1"
	"github.com/google/go-containerregistry/pkg/v1/empty"
	"github.com/google/go-containerregistry/pkg/v1/layout"
	"github.com/google/go-containerregistry/pkg/v1/tarball"
	"github.com/google/go-containerregistry/pkg/v1/validate"
)

const limit = 10 * 1024 * 1024 * 1024

type identity struct {
	Name         string `json:"name"`
	Manifest     string `json:"manifest"`
	Config       string `json:"config"`
	OS           string `json:"os"`
	Architecture string `json:"architecture"`
}

// The native reader never extracts layers. Bound and validate its transport
// archive before letting it seek members, including duplicate/path aliases.
func inspectArchive(filename string) error {
	f, err := os.Open(filename)
	if err != nil {
		return err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil {
		return err
	}
	if !info.Mode().IsRegular() || info.Size() > limit {
		return fmt.Errorf("image transport exceeds limit or is not regular")
	}
	archive := tar.NewReader(f)
	names := map[string]bool{}
	var total int64
	for {
		entry, err := archive.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		name := strings.TrimSuffix(entry.Name, "/")
		if name == "" || strings.HasPrefix(name, "/") || path.Clean(name) != name || strings.Contains(name, "\\") || name == ".." || strings.HasPrefix(name, "../") || names[name] || (len(name) > 1 && name[1] == ':') {
			return fmt.Errorf("invalid image transport member: %q", name)
		}
		if entry.Typeflag != tar.TypeReg && entry.Typeflag != tar.TypeDir {
			return fmt.Errorf("image transport links are unsupported")
		}
		names[name] = true
		total += entry.Size
		if total > limit || len(names) > 100000 {
			return fmt.Errorf("image transport budget exceeded")
		}
	}
}

func capture(input, output, expected, platform, requested string) (identity, error) {
	result := identity{}
	named, err := reference.ParseNormalizedNamed(requested)
	if err != nil {
		return result, err
	}
	contextName := strings.TrimSuffix(reference.FamiliarString(named), ":latest")
	if err := inspectArchive(input); err != nil {
		return result, err
	}
	image, err := tarball.ImageFromPath(input, nil)
	if err != nil {
		return result, err
	}
	config, err := image.ConfigName()
	if err != nil {
		return result, err
	}
	if config.String() != expected {
		return result, fmt.Errorf("exported image differs from inspected identity")
	}
	facts, err := image.ConfigFile()
	if err != nil {
		return result, err
	}
	if facts.OS+"/"+facts.Architecture != platform {
		return result, fmt.Errorf("base image platform differs from build")
	}
	if len(facts.Config.OnBuild) != 0 {
		return result, fmt.Errorf("base ONBUILD requires captured-input discovery integration")
	}
	if err := validate.Image(image); err != nil {
		return result, fmt.Errorf("invalid native image: %w", err)
	}
	if err := os.Mkdir(output, 0755); err != nil {
		return result, err
	}
	store, err := layout.Write(output, empty.Index)
	if err != nil {
		return result, err
	}
	// Preserve native config/layer bytes and media types; BuildKit's OCI store
	// supports both OCI and Docker schema-2 descriptors.
	if err := store.AppendImage(image, layout.WithPlatform(v1.Platform{OS: facts.OS, Architecture: facts.Architecture, Variant: facts.Variant})); err != nil {
		return result, err
	}
	digest, err := image.Digest()
	if err != nil {
		return result, err
	}
	return identity{contextName, digest.String(), config.String(), facts.OS, facts.Architecture}, nil
}

func main() {
	if len(os.Args) != 6 {
		fmt.Fprintln(os.Stderr, "usage: oyzu-docker-images <docker-save> <new-layout> <config-digest> <os/arch> <reference>")
		os.Exit(2)
	}
	result, err := capture(os.Args[1], os.Args[2], os.Args[3], os.Args[4], os.Args[5])
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
	if err := json.NewEncoder(os.Stdout).Encode(result); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
