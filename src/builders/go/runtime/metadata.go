// Native Go model capture. This program reads manifests and invokes Go metadata
// commands; it never builds or runs project packages. The caller owns isolation.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
)

type moduleFile struct {
	Module  struct{ Path string }
	Use     []struct{ DiskPath string }
	Replace []struct {
		New struct{ Path, Version string }
	}
}
type nativePackage struct {
	Dir, ImportPath, Name, Target, ForTest string
	Standard                               bool
	CgoFiles                               []string
	Module                                 *module
}
type binary struct {
	Name      string `json:"name"`
	Package   string `json:"package"`
	Directory string `json:"directory"`
}
type inventory struct {
	Version        string       `json:"version"`
	OS             string       `json:"os"`
	Arch           string       `json:"arch"`
	Patterns       []string     `json:"patterns"`
	Modules        []string     `json:"modules"`
	Binaries       []binary     `json:"binaries"`
	Cgo            bool         `json:"cgo"`
	Compiler       string       `json:"compiler,omitempty"`
	CompilerTarget string       `json:"compilerTarget,omitempty"`
	Dependencies   []dependency `json:"dependencies"`
}

func run(dir, program string, args ...string) ([]byte, error) {
	cmd := exec.Command(program, args...)
	cmd.Dir = dir
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	out, err := cmd.Output()
	if err != nil {
		return nil, fmt.Errorf("%s %v: %w\n%s", program, args, err, stderr.String())
	}
	return out, nil
}

// Reject a local reference before asking Go to load it. Resolve symlinks as well
// as lexical traversal; a native manifest cannot expand the captured input root.
func contained(root, base, path string) (string, error) {
	if !filepath.IsAbs(path) {
		path = filepath.Join(base, path)
	}
	resolved, err := filepath.EvalSymlinks(path)
	if err != nil {
		return "", err
	}
	rel, err := filepath.Rel(root, resolved)
	if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) || filepath.IsAbs(rel) {
		return "", fmt.Errorf("Go input outside captured target: %s", path)
	}
	return filepath.ToSlash(rel), nil
}

func manifest(root, dir, kind string) (moduleFile, error) {
	var value moduleFile
	out, err := run(dir, "go", kind, "edit", "-json")
	if err != nil {
		return value, err
	}
	if err = json.Unmarshal(out, &value); err != nil {
		return value, err
	}
	for _, replacement := range value.Replace {
		if replacement.New.Version == "" {
			if _, err = contained(root, dir, replacement.New.Path); err != nil {
				return value, err
			}
		}
	}
	return value, nil
}

func capture(root string, acquire *acquisitionOptions) (inventory, error) {
	v := inventory{Patterns: []string{}, Modules: []string{}, Binaries: []binary{}, Dependencies: []dependency{}}
	var err error
	root, err = filepath.EvalSymlinks(root)
	if err != nil {
		return v, err
	}
	// Never discover an ancestor's workspace or inherit user Go configuration.
	work := filepath.Join(root, "go.work")
	if _, err = os.Stat(work); os.IsNotExist(err) {
		work = "off"
	} else if err != nil {
		return v, err
	}
	os.Setenv("GOWORK", work)
	os.Setenv("GOENV", "off")
	os.Setenv("GOTOOLCHAIN", "local")
	os.Setenv("GOPROXY", "off")
	os.Setenv("GOSUMDB", "off")
	os.Setenv("GOFLAGS", "-mod=readonly -p=2")
	modules := []string{"."}
	if work != "off" {
		model, err := manifest(root, root, "work")
		if err != nil {
			return v, err
		}
		modules = nil
		for _, use := range model.Use {
			path, err := contained(root, root, use.DiskPath)
			if err != nil {
				return v, err
			}
			modules = append(modules, path)
		}
	}
	sort.Strings(modules)
	if len(modules) == 0 {
		return v, fmt.Errorf("Go workspace has no modules")
	}
	seen := map[string]bool{}
	localModules := []string{}
	for _, module := range modules {
		if seen[module] {
			return v, fmt.Errorf("duplicate Go workspace member %s", module)
		}
		seen[module] = true
		model, err := manifest(root, filepath.Join(root, module), "mod")
		if err != nil {
			return v, err
		}
		localModules = append(localModules, model.Module.Path)
		v.Modules = append(v.Modules, module)
		pattern := "./..."
		if module != "." {
			pattern = "./" + module + "/..."
		}
		v.Patterns = append(v.Patterns, pattern)
	}
	var session *acquisition
	if acquire != nil {
		session, err = beginAcquisition(root, v.Modules, localModules, *acquire)
		if err != nil {
			return v, err
		}
		defer session.server.Close()
	}
	var env map[string]string
	out, err := run(root, "go", "env", "-json", "GOVERSION", "GOOS", "GOARCH", "CGO_ENABLED", "CC")
	if err != nil {
		return v, err
	}
	if err = json.Unmarshal(out, &env); err != nil {
		return v, err
	}
	v.Version, v.OS, v.Arch = env["GOVERSION"], env["GOOS"], env["GOARCH"]
	out, err = run(root, "go", append([]string{"list", "-json"}, v.Patterns...)...)
	if err != nil {
		return v, err
	}
	decoder := json.NewDecoder(bytes.NewReader(out))
	for {
		var p nativePackage
		if err = decoder.Decode(&p); err == io.EOF {
			break
		} else if err != nil {
			return v, err
		}
		dir, err := contained(root, root, p.Dir)
		if err != nil {
			return v, err
		}
		if p.Name == "main" {
			if p.Target == "" {
				return v, fmt.Errorf("native binary target missing for %s", p.ImportPath)
			}
			name := filepath.Base(p.Target)
			if v.OS == "windows" {
				name = strings.TrimSuffix(name, ".exe")
			}
			v.Binaries = append(v.Binaries, binary{name, p.ImportPath, dir})
		}
	}
	// Include test-only dependencies in admission and compiler requirement checks.
	out, err = run(root, "go", append([]string{"list", "-deps", "-test", "-json"}, v.Patterns...)...)
	if err != nil {
		return v, fmt.Errorf("Go dependency metadata: %w", err)
	}
	decoder = json.NewDecoder(bytes.NewReader(out))
	external := map[string]module{}
	for {
		var p nativePackage
		if err = decoder.Decode(&p); err == io.EOF {
			break
		} else if err != nil {
			return v, err
		}
		if !p.Standard {
			if _, err = contained(root, root, p.Dir); err != nil {
				if session == nil || p.Module == nil {
					return v, err
				}
				if _, err = contained(session.options.cache, session.options.cache, p.Dir); err != nil {
					return v, err
				}
				m := *p.Module
				if m.Replace != nil {
					m = *m.Replace
				}
				if m.Version == "" || m.Main {
					return v, fmt.Errorf("uncaptured local Go dependency %s", m.Path)
				}
				external[m.Path+"@"+m.Version] = m
			}
		}
		if len(p.CgoFiles) > 0 {
			v.Cgo = true
		}
	}
	if session != nil {
		v.Dependencies, err = session.finish(root, external)
		if err != nil {
			return v, err
		}
	}
	if v.Cgo {
		if env["CGO_ENABLED"] != "1" {
			return v, fmt.Errorf("cgo requires CGO_ENABLED=1")
		}
		// The provisioned toolchain profile uses one native compiler executable.
		if strings.ContainsAny(env["CC"], " \t") {
			return v, fmt.Errorf("compiler wrappers are not supported by this Go profile")
		}
		out, err = run(root, env["CC"], "--version")
		if err != nil {
			return v, fmt.Errorf("cgo compiler unavailable: %w", err)
		}
		v.Compiler = strings.TrimSpace(string(out))
		out, err = run(root, env["CC"], "-dumpmachine")
		if err != nil {
			return v, err
		}
		v.CompilerTarget = strings.TrimSpace(string(out))
	}
	sort.Slice(v.Binaries, func(i, j int) bool { return v.Binaries[i].Package < v.Binaries[j].Package })
	return v, nil
}

func main() {
	if len(os.Args) != 2 && len(os.Args) != 4 {
		fmt.Fprintln(os.Stderr, "usage: go-metadata OUTPUT [MODULE_CACHE BROKER]")
		os.Exit(2)
	}
	root, err := os.Getwd()
	var v inventory
	var acquire *acquisitionOptions
	if len(os.Args) == 4 {
		acquire = &acquisitionOptions{os.Args[2], os.Args[3]}
	}
	if err == nil {
		v, err = capture(root, acquire)
	}
	var data []byte
	if err == nil {
		data, err = json.MarshalIndent(v, "", "  ")
	}
	if err == nil {
		err = os.WriteFile(os.Args[1], append(data, '\n'), 0600)
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
