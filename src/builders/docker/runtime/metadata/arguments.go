package main

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/containerd/platforms"
	"github.com/moby/buildkit/frontend/dockerfile/instructions"
	"github.com/moby/buildkit/frontend/dockerfile/shell"
)

// Selection facts are supplied by the executor contract, never ambient state.
type selectionFacts struct {
	TargetPlatform  string `json:"targetPlatform"`
	SourceDateEpoch string `json:"sourceDateEpoch"`
}

// Worker facts are not inferred from target facts: a worker may execute another
// platform. Native expansion must not silently treat these built-ins as absent.
func reservedArgument(name string) bool {
	switch name {
	case "BUILDPLATFORM", "BUILDOS", "BUILDOSVERSION", "BUILDARCH", "BUILDVARIANT":
		return true
	}
	return false
}

func expandSelection(lex *shell.Lex, word string, environment shell.EnvGetter) (string, error) {
	result, err := lex.ProcessWordWithMatches(word, environment)
	if err != nil {
		return "", err
	}
	for _, references := range []map[string]struct{}{result.Matched, result.Unmatched} {
		for key := range references {
			if reservedArgument(key) {
				return "", fmt.Errorf("image selection using %s requires captured automatic argument integration", key)
			}
		}
	}
	return result.Result, nil
}

func globalDefaults(lex *shell.Lex, declarations []instructions.ArgCommand, facts selectionFacts, finalStage string) (shell.EnvGetter, error) {
	platform, err := platforms.Parse(facts.TargetPlatform)
	if err != nil {
		return nil, fmt.Errorf("invalid captured target platform: %w", err)
	}
	epoch, err := strconv.ParseInt(facts.SourceDateEpoch, 10, 64)
	if err != nil || epoch < 0 || strconv.FormatInt(epoch, 10) != facts.SourceDateEpoch {
		return nil, fmt.Errorf("invalid captured source date epoch")
	}
	if finalStage == "" {
		finalStage = "default"
	}
	values := []string{
		"TARGETPLATFORM=" + platforms.FormatAll(platform),
		"TARGETOS=" + platform.OS, "TARGETOSVERSION=" + platform.OSVersion,
		"TARGETARCH=" + platform.Architecture, "TARGETVARIANT=" + platform.Variant,
		"TARGETSTAGE=" + finalStage,
	}
	for _, declaration := range declarations {
		for _, argument := range declaration.Args {
			if argument.Value == nil && argument.Key != "SOURCE_DATE_EPOCH" {
				continue
			}
			// This build argument is always supplied by the executor, so it
			// overrides a Dockerfile default only when globally declared.
			value := facts.SourceDateEpoch
			if argument.Key != "SOURCE_DATE_EPOCH" {
				value, err = expandSelection(lex, *argument.Value, shell.EnvsFromSlice(values))
				if err != nil {
					return nil, err
				}
			}
			// Redeclaration replaces the prior default, including an empty one.
			prefix := argument.Key + "="
			updated := values[:0]
			for _, existing := range values {
				if !strings.HasPrefix(existing, prefix) {
					updated = append(updated, existing)
				}
			}
			values = append(updated, prefix+value)
		}
	}
	return shell.EnvsFromSlice(values), nil
}
