// Credential-free Go client for the engine-owned scoped acquisition channel.
package main

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"time"
)

func brokerGET(spool, url string) (int, string, []byte, error) {
	var token [16]byte
	if _, err := rand.Read(token[:]); err != nil {
		return 0, "", nil, err
	}
	id := hex.EncodeToString(token[:])
	pending := filepath.Join(spool, id+".pending")
	data, _ := json.Marshal(map[string]string{"url": url})
	request, err := os.OpenFile(pending, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if err != nil {
		return 0, "", nil, err
	}
	_, err = request.Write(data)
	closed := request.Close()
	if err != nil {
		return 0, "", nil, err
	}
	if closed != nil {
		return 0, "", nil, closed
	}
	if err := os.Rename(pending, filepath.Join(spool, id+".request")); err != nil {
		return 0, "", nil, err
	}
	response := filepath.Join(spool, id+".response")
	deadline := time.Now().Add(55 * time.Second)
	for {
		data, err := os.ReadFile(response)
		if err == nil {
			var meta struct {
				Status      int
				ContentType string
			}
			if err = json.Unmarshal(data, &meta); err != nil {
				return 0, "", nil, err
			}
			bodyPath := filepath.Join(spool, id+".body")
			file, err := os.Open(bodyPath)
			if err != nil {
				return 0, "", nil, err
			}
			body, err := io.ReadAll(io.LimitReader(file, 128*1024*1024+1))
			file.Close()
			os.Remove(response)
			os.Remove(bodyPath)
			if len(body) > 128*1024*1024 {
				return 0, "", nil, fmt.Errorf("broker response exceeds limit")
			}
			return meta.Status, meta.ContentType, body, err
		}
		if !os.IsNotExist(err) {
			return 0, "", nil, err
		}
		if time.Now().After(deadline) {
			return 0, "", nil, fmt.Errorf("scoped acquisition broker did not respond")
		}
		time.Sleep(10 * time.Millisecond)
	}
}
