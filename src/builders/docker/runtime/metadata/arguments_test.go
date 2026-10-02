package main

import (
	"strings"
	"testing"
)

func TestGlobalDefaultsResolveNativeImageAndPlatformSelection(t *testing.T) {
	t.Setenv("BASE", "ambient.invalid/not-an-input:1")
	for _, test := range []struct{ name, body, base, platform string }{
		{"simple", "ARG BASE=alpine:3.22\nFROM ${BASE}\n", "alpine:3.22", ""},
		{"composed", "ARG REGISTRY=docker.io/library\nARG VERSION=3.22\nARG BASE=${REGISTRY}/alpine:${VERSION}\nFROM $BASE\n", "docker.io/library/alpine:3.22", ""},
		{"native-default", "ARG BASE\nFROM ${BASE:-alpine:3.22}\n", "alpine:3.22", ""},
		{"redeclaration", "ARG BASE=alpine:3.21\nARG BASE=alpine:3.22\nARG BASE\nFROM ${BASE}\n", "alpine:3.22", ""},
		{"empty-default", "ARG BASE=alpine:3.21\nARG BASE=\nFROM ${BASE:-alpine:3.22}\n", "alpine:3.22", ""},
		{"quoted", "ARG BASE=\"alpine:3.22\"\nFROM $BASE\n", "alpine:3.22", ""},
		{"platform", "ARG PLATFORM=linux/amd64\nARG BASE=alpine:3.22\nFROM --platform=$PLATFORM $BASE\n", "alpine:3.22", "linux/amd64"},
	} {
		t.Run(test.name, func(t *testing.T) {
			metadata, err := analyze([]byte(test.body))
			if err != nil {
				t.Fatal(err)
			}
			if metadata.Stages[0].Base != test.base || metadata.Stages[0].Platform != test.platform {
				t.Fatalf("wrong native stage: %+v", metadata.Stages[0])
			}
			images := 0
			for _, requirement := range metadata.Requirements {
				if requirement.Kind == "image" && requirement.Reference == test.base {
					images++
				} else if requirement.Kind != "platform" || requirement.Reference != test.platform {
					t.Fatalf("unexpected input: %+v", requirement)
				}
			}
			if images != 1 {
				t.Fatal("resolved image was not declared exactly once")
			}
		})
	}
}

func TestResolvedScratchAndStageAliasesDoNotBecomeImageInputs(t *testing.T) {
	metadata, err := analyze([]byte("ARG BASE=scratch\nARG STAGE=content\nFROM $BASE AS content\nCOPY <<EOF /file\npayload\nEOF\nFROM $STAGE\n"))
	if err != nil || len(metadata.Requirements) != 0 {
		t.Fatalf("resolved local stages became external inputs: %+v, %v", metadata, err)
	}
}

func TestSelectionRejectsUncapturedAutomaticArgumentsAndEmptyValues(t *testing.T) {
	t.Setenv("BASE", "alpine:3.22")
	for _, body := range []string{
		"ARG BASE\nFROM $BASE\n",
		"FROM scratch\nARG BASE=alpine:3.22\nFROM $BASE\n",
		"ARG PLATFORM\nFROM --platform=$PLATFORM scratch\n",
	} {
		if _, err := analyze([]byte(body)); err == nil || !strings.Contains(err.Error(), "empty") {
			t.Fatalf("accepted missing global input: %q: %v", body, err)
		}
	}
	for _, body := range []string{
		"FROM example.invalid/base:$TARGETARCH\n",
		"ARG BASE=example.invalid/base:$BUILDARCH\nFROM $BASE\n",
		"FROM --platform=${TARGETPLATFORM:-linux/amd64} scratch\n",
		"ARG SOURCE_DATE_EPOCH=1\nARG BASE=example.invalid/base:$SOURCE_DATE_EPOCH\nFROM $BASE\n",
	} {
		if _, err := analyze([]byte(body)); err == nil || !strings.Contains(err.Error(), "captured automatic argument") {
			t.Fatalf("guessed automatic inputs: %q: %v", body, err)
		}
	}
}
