package main

import (
	"fmt"
	"strings"

	"github.com/moby/buildkit/frontend/dockerfile/instructions"
	"github.com/moby/buildkit/frontend/dockerfile/shell"
)

// Image selection uses only declared global defaults. Native expansion handles
// quoting, escapes and parameter operators; no process environment is consulted.
// Automatic/platform and executor-supplied arguments need captured facts before
// they can participate in selection, even when a fallback is written for them.
func reservedArgument(name string) bool {
	switch name {
	case "BUILDPLATFORM", "BUILDOS", "BUILDOSVERSION", "BUILDARCH", "BUILDVARIANT",
		"TARGETPLATFORM", "TARGETOS", "TARGETOSVERSION", "TARGETARCH", "TARGETVARIANT",
		"TARGETSTAGE", "SOURCE_DATE_EPOCH":
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

func globalDefaults(lex *shell.Lex, declarations []instructions.ArgCommand) (shell.EnvGetter, error) {
	values := []string{}
	for _, declaration := range declarations {
		for _, argument := range declaration.Args {
			if argument.Value == nil {
				continue
			}
			value, err := expandSelection(lex, *argument.Value, shell.EnvsFromSlice(values))
			if err != nil {
				return nil, err
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
