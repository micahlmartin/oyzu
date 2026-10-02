// Native module archive projection over captured source. This tool has no
// network or credential interface; x/mod owns module paths and zip semantics.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"golang.org/x/mod/modfile"
	"golang.org/x/mod/module"
	"golang.org/x/mod/semver"
	"golang.org/x/mod/sumdb/dirhash"
	modzip "golang.org/x/mod/zip"
)

type metadata struct {
	Modules            []string            `json:"modules"`
	Binaries           []json.RawMessage   `json:"binaries"`
	ModuleDependencies map[string][]string `json:"moduleDependencies"`
}

type artifact struct {
	Path      string `json:"path"`
	Directory string `json:"directory"`
	Version   string `json:"version"`
	Zip       string `json:"zip"`
	Mod       string `json:"mod"`
	Info      string `json:"info"`
	Sum       string `json:"sum"`
	GoModSum  string `json:"goModSum"`
}

func archive(root, input, baseVersion, mode, output string) error {
	output, err := filepath.Abs(output)
	if err != nil {
		return err
	}
	if relative, err := filepath.Rel(root, output); err == nil && relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return fmt.Errorf("module artifacts must be outside captured source")
	}
	data, err := os.ReadFile(input)
	if err != nil {
		return err
	}
	var facts metadata
	if err = json.Unmarshal(data, &facts); err != nil {
		return err
	}
	if mode != "go/app" && mode != "go/library" {
		return fmt.Errorf("unknown Go builder %s", mode)
	}
	artifacts := []artifact{}
	if mode == "go/app" && len(facts.Binaries) != 0 {
		return writeInventory(output, artifacts)
	}
	if len(facts.Modules) == 0 {
		return fmt.Errorf("no native modules to package")
	}
	baseVersion = "v" + strings.TrimPrefix(baseVersion, "v")
	if !semver.IsValid(baseVersion) || !strings.Contains(semver.Prerelease(baseVersion), "-dev.g") {
		return fmt.Errorf("module packaging requires a snapshot version")
	}
	models := []*modfile.File{}
	roots := []string{}
	versions := map[string]string{}
	for _, dir := range facts.Modules {
		path, err := contained(root, dir)
		if err != nil {
			return err
		}
		data, err := os.ReadFile(filepath.Join(path, "go.mod"))
		if err != nil {
			return err
		}
		if len(data) > modzip.MaxGoMod {
			return fmt.Errorf("go.mod exceeds native limit")
		}
		model, err := modfile.Parse("go.mod", data, nil)
		if err != nil {
			return err
		}
		if model.Module == nil {
			return fmt.Errorf("missing module path")
		}
		name := model.Module.Mod.Path
		if _, duplicate := versions[name]; duplicate {
			return fmt.Errorf("duplicate module %s", name)
		}
		_, major, ok := module.SplitPathVersion(name)
		if !ok {
			return fmt.Errorf("invalid module path %s", name)
		}
		version := baseVersion
		if major != "" && semver.Major(baseVersion) != module.PathMajorPrefix(major) {
			version = module.PathMajorPrefix(major) + ".0.0" + semver.Prerelease(baseVersion)
		}
		if err = module.Check(name, version); err != nil {
			return err
		}
		versions[name] = version
		roots, models = append(roots, path), append(models, model)
	}
	for index, model := range models {
		if data, err := os.ReadFile(filepath.Join(root, "go.work")); err == nil {
			work, err := modfile.ParseWork("go.work", data, nil)
			if err != nil {
				return err
			}
			for _, replacement := range work.Replace {
				if replacement.New.Version != "" {
					return fmt.Errorf("publishing workspace registry replacements requires an explicit module projection")
				}
				path, err := contained(root, replacement.New.Path)
				if err != nil {
					return err
				}
				rel, err := filepath.Rel(roots[index], path)
				if err != nil {
					return err
				}
				if err = model.AddReplace(replacement.Old.Path, replacement.Old.Version, filepath.ToSlash(rel), ""); err != nil {
					return err
				}
			}
		} else if !os.IsNotExist(err) {
			return err
		}
		// Workspace and contained local replacement dependencies are published at
		// this build's module versions, never as machine-local paths.
		for _, replacement := range append([]*modfile.Replace{}, model.Replace...) {
			if replacement.New.Version != "" {
				return fmt.Errorf("publishing registry replacements requires an explicit module projection")
			}
			path, err := contained(root, filepath.Join(facts.Modules[index], replacement.New.Path))
			if err != nil {
				return err
			}
			known := false
			for other, candidate := range roots {
				if path == candidate && replacement.Old.Path == models[other].Module.Mod.Path {
					known = true
				}
			}
			if !known {
				return fmt.Errorf("local replacement %s is not a captured workspace module with the same identity", replacement.Old.Path)
			}
			if err = model.DropReplace(replacement.Old.Path, replacement.Old.Version); err != nil {
				return err
			}
		}
		for _, require := range append([]*modfile.Require{}, model.Require...) {
			if version, local := versions[require.Mod.Path]; local {
				if err = model.AddRequire(require.Mod.Path, version); err != nil {
					return err
				}
			}
		}
		for _, dependency := range facts.ModuleDependencies[model.Module.Mod.Path] {
			version, known := versions[dependency]
			if !known {
				return fmt.Errorf("unknown native workspace dependency %s", dependency)
			}
			if err = model.AddRequire(dependency, version); err != nil {
				return err
			}
		}
		model.Cleanup()
		projected, err := model.Format()
		if err != nil {
			return err
		}
		checked, err := modzip.CheckDir(roots[index])
		if err != nil {
			return err
		}
		if err = checked.Err(); err != nil {
			return err
		}
		files := []modzip.File{}
		for _, path := range checked.Valid {
			rel, err := filepath.Rel(roots[index], path)
			if err != nil {
				return err
			}
			file := sourceFile{path: path, name: filepath.ToSlash(rel)}
			if file.name == "go.mod" {
				file.data = projected
			}
			files = append(files, file)
		}
		name := model.Module.Mod.Path
		version := versions[name]
		stem := fmt.Sprintf("module-%d-%s", index, version)
		item := artifact{Path: name, Directory: facts.Modules[index], Version: version, Zip: stem + ".zip", Mod: stem + ".mod", Info: stem + ".info"}
		if err = os.MkdirAll(output, 0700); err != nil {
			return err
		}
		archivePath := filepath.Join(output, item.Zip)
		stream, err := os.OpenFile(archivePath, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if err != nil {
			return err
		}
		err = modzip.Create(stream, module.Version{Path: name, Version: version}, files)
		closed := stream.Close()
		if err != nil {
			return err
		}
		if closed != nil {
			return closed
		}
		if _, err = modzip.CheckZip(module.Version{Path: name, Version: version}, archivePath); err != nil {
			return err
		}
		item.Sum, err = dirhash.HashZip(archivePath, dirhash.Hash1)
		if err != nil {
			return err
		}
		item.GoModSum, err = dirhash.Hash1([]string{"go.mod"}, func(string) (io.ReadCloser, error) { return io.NopCloser(bytes.NewReader(projected)), nil })
		if err != nil {
			return err
		}
		if err = os.WriteFile(filepath.Join(output, item.Mod), projected, 0600); err != nil {
			return err
		}
		info, _ := json.Marshal(struct{ Version string }{version})
		if err = os.WriteFile(filepath.Join(output, item.Info), append(info, '\n'), 0600); err != nil {
			return err
		}
		artifacts = append(artifacts, item)
	}
	return writeInventory(output, artifacts)
}

func writeInventory(output string, artifacts []artifact) error {
	if err := os.MkdirAll(output, 0700); err != nil {
		return err
	}
	data, err := json.MarshalIndent(artifacts, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(output, "inventory.json"), append(data, '\n'), 0600)
}
