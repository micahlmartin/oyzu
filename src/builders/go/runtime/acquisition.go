// Native module acquisition through the engine's credential-free spool. Only
// preparation starts this proxy; action execution receives the frozen cache.
package main

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net"
	"net/http"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"
)

type acquisitionOptions struct{ cache, broker string }
type acquisition struct {
	options acquisitionOptions
	server  *http.Server
	locks   map[string]sourceLock
}
type sourceLock struct {
	data   []byte
	exists bool
}
type module struct {
	Path, Version, Dir string
	Main               bool
	Replace            *module
}
type dependency struct {
	Name     string `json:"name"`
	Version  string `json:"version"`
	File     string `json:"file"`
	Digest   string `json:"digest"`
	Size     int    `json:"size"`
	Sum      string `json:"sum"`
	GoModSum string `json:"goModSum"`
}

func beginAcquisition(root string, members, localModules []string, options acquisitionOptions) (*acquisition, error) {
	cache, err := filepath.Abs(options.cache)
	if err != nil {
		return nil, err
	}
	options.cache = cache
	if err = os.MkdirAll(cache, 0700); err != nil {
		return nil, err
	}
	a := &acquisition{options: options, locks: map[string]sourceLock{}}
	// Go can probe a required version's .mod even when the workspace owns that
	// module. Answer locally rather than disclosing workspace names upstream.
	// The native proxy protocol encodes uppercase ASCII as ! followed by lowercase.
	local := map[string]bool{}
	for _, name := range localModules {
		var escaped strings.Builder
		for _, ch := range name {
			if ch >= 'A' && ch <= 'Z' {
				escaped.WriteRune('!')
				ch += 'a' - 'A'
			}
			escaped.WriteRune(ch)
		}
		local["/"+escaped.String()] = true
	}
	files := []string{filepath.Join(root, "go.work"), filepath.Join(root, "go.work.sum")}
	for _, member := range members {
		files = append(files, filepath.Join(root, member, "go.mod"), filepath.Join(root, member, "go.sum"))
	}
	for _, file := range files {
		data, err := os.ReadFile(file)
		if err != nil && !os.IsNotExist(err) {
			return nil, err
		}
		a.locks[file] = sourceLock{data, err == nil}
	}
	listener, err := net.Listen("tcp4", "127.0.0.1:0")
	if err != nil {
		return nil, err
	}
	a.server = &http.Server{ReadHeaderTimeout: 5 * time.Second, Handler: http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" || r.URL.RawQuery != "" || strings.Contains(r.URL.Path, "..") {
			http.Error(w, "unsupported module request", 400)
			return
		}
		if modulePath, _, ok := strings.Cut(r.URL.Path, "/@"); ok && local[modulePath] {
			http.Error(w, "module belongs to the captured workspace", http.StatusNotFound)
			return
		}
		status, contentType, body, err := brokerGET(options.broker, "https://proxy.golang.org"+r.URL.EscapedPath())
		if err != nil {
			http.Error(w, "module acquisition failed", 502)
			return
		}
		w.Header().Set("Content-Type", contentType)
		w.WriteHeader(status)
		w.Write(body)
	})}
	go a.server.Serve(listener)
	os.Setenv("GOMODCACHE", cache)
	os.Setenv("GOPROXY", "http://"+listener.Addr().String())
	os.Setenv("GOPRIVATE", "")
	os.Setenv("GONOPROXY", "")
	os.Setenv("GONOSUMDB", "")
	os.Setenv("GOVCS", "*:off")
	return a, nil
}

func (a *acquisition) finish(root string, modules map[string]module) ([]dependency, error) {
	dependencies := []dependency{}
	names := make([]string, 0, len(modules))
	for name := range modules {
		names = append(names, name)
	}
	sort.Strings(names)
	for _, name := range names {
		m := modules[name]
		out, err := run(root, "go", "mod", "download", "-json", m.Path+"@"+m.Version)
		if err != nil {
			return nil, err
		}
		var item struct{ Path, Version, Zip, Sum, GoModSum, Error string }
		if err = json.Unmarshal(out, &item); err != nil {
			return nil, err
		}
		if item.Error != "" || item.Sum == "" || item.GoModSum == "" {
			return nil, fmt.Errorf("missing native Go checksum evidence for %s", name)
		}
		rel, err := contained(a.options.cache, a.options.cache, item.Zip)
		if err != nil {
			return nil, err
		}
		body, err := os.ReadFile(item.Zip)
		if err != nil {
			return nil, err
		}
		digest := sha256.Sum256(body)
		dependencies = append(dependencies, dependency{item.Path, item.Version, rel, "sha256:" + hex.EncodeToString(digest[:]), len(body), item.Sum, item.GoModSum})
	}
	for file, original := range a.locks {
		current, err := os.ReadFile(file)
		if err != nil && !os.IsNotExist(err) {
			return nil, err
		}
		if original.exists != (err == nil) || !bytes.Equal(original.data, current) {
			return nil, fmt.Errorf("native Go resolution changed %s; update and commit native manifests/checksums before building", filepath.Base(file))
		}
	}
	// Native cache lists are sets; normalize concurrent append order only.
	err := filepath.WalkDir(filepath.Join(a.options.cache, "cache", "download"), func(path string, entry os.DirEntry, err error) error {
		if os.IsNotExist(err) {
			return nil
		}
		if err != nil {
			return err
		}
		if !entry.IsDir() && entry.Name() == "list" && filepath.Base(filepath.Dir(path)) == "@v" {
			data, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			lines := strings.Fields(string(data))
			sort.Strings(lines)
			return os.WriteFile(path, []byte(strings.Join(lines, "\n")+"\n"), 0600)
		}
		return nil
	})
	return dependencies, err
}
