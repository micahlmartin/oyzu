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
			metadata, err := analyzeFixture([]byte(test.body))
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
	metadata, err := analyzeFixture([]byte("ARG BASE=scratch\nARG STAGE=content\nFROM $BASE AS content\nCOPY <<EOF /file\npayload\nEOF\nFROM $STAGE\n"))
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
		if _, err := analyzeFixture([]byte(body)); err == nil || !strings.Contains(err.Error(), "empty") {
			t.Fatalf("accepted missing global input: %q: %v", body, err)
		}
	}
	for _, body := range []string{
		"FROM example.invalid/base:$BUILDARCH\n",
		"ARG BASE=example.invalid/base:$BUILDARCH\nFROM $BASE\n",
		"FROM --platform=${BUILDPLATFORM:-linux/amd64} scratch\n",
	} {
		if _, err := analyzeFixture([]byte(body)); err == nil || !strings.Contains(err.Error(), "captured automatic argument") {
			t.Fatalf("guessed automatic inputs: %q: %v", body, err)
		}
	}
}

func TestTargetArgumentsUseCapturedFactsAndNativeScope(t *testing.T) {
	t.Setenv("TARGETARCH", "ambient-architecture")
	t.Setenv("SOURCE_DATE_EPOCH", "42")
	for _, test := range []struct{ body, base, platform string }{
		{"FROM example.invalid/${TARGETOS}/${TARGETARCH}:${TARGETVARIANT:-default}\n", "example.invalid/linux/arm64:default", ""},
		{"FROM --platform=$TARGETPLATFORM scratch\n", "scratch", "linux/arm64"},
		{"FROM example.invalid/$TARGETSTAGE:1 AS final\n", "example.invalid/final:1", ""},
		{"FROM example.invalid/$TARGETSTAGE:1\n", "example.invalid/default:1", ""},
		{"ARG TARGETARCH=custom\nFROM example.invalid/$TARGETARCH:1\n", "example.invalid/custom:1", ""},
		{"ARG TARGETARCH\nFROM example.invalid/$TARGETARCH:1\n", "example.invalid/arm64:1", ""},
		{"ARG SOURCE_DATE_EPOCH=1\nFROM example.invalid/base:$SOURCE_DATE_EPOCH\n", "example.invalid/base:315532800", ""},
		{"ARG SOURCE_DATE_EPOCH\nFROM example.invalid/base:$SOURCE_DATE_EPOCH\n", "example.invalid/base:315532800", ""},
		{"FROM example.invalid/base:${SOURCE_DATE_EPOCH:-undeclared}\n", "example.invalid/base:undeclared", ""},
	} {
		facts := selectionFacts{"linux/arm64", "315532800"}
		metadata, err := analyze([]byte(test.body), facts)
		if err != nil {
			t.Fatal(err)
		}
		if metadata.Selection != facts || metadata.Stages[0].Base != test.base || metadata.Stages[0].Platform != test.platform {
			t.Fatalf("wrong captured expansion for %q: %+v", test.body, metadata)
		}
	}
	variant, err := analyze([]byte("FROM example.invalid/$TARGETARCH:$TARGETVARIANT\n"), selectionFacts{"linux/arm/v7", "315532800"})
	if err != nil || variant.Stages[0].Base != "example.invalid/arm:v7" {
		t.Fatalf("lost native platform variant: %+v, %v", variant, err)
	}
	for _, facts := range []selectionFacts{{"not a platform", "315532800"}, {"linux/amd64", "-1"}, {"linux/amd64", "0; command"}} {
		if _, err := analyze([]byte("FROM scratch\n"), facts); err == nil {
			t.Fatalf("accepted invalid executor facts: %+v", facts)
		}
	}
}
