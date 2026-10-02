"""Validate draft build-record fixtures; never execute or certify an Oyzu build."""
import json
import re
from pathlib import Path, PurePosixPath

try:
    from jsonschema import Draft202012Validator, FormatChecker
    import yaml
except ImportError:
    raise SystemExit("Install tooling/design-requirements.txt in the validation environment.")

ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "docs/contracts/v1alpha1"


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("Duplicate JSON key: " + key)
        result[key] = value
    return result


def read(path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_pairs)


def semantics(kind, value):
    """Small documented invariant checks; not a replacement for the future engine."""
    errors = set()

    def indexed(items):
        mapped = {item["id"]: item for item in items}
        if len(mapped) != len(items):
            errors.add("duplicates")
        return mapped

    def cycle(graph):
        active, complete = set(), set()

        def visit(node):
            if node in active:
                errors.add("cycle")
                return
            if node in complete:
                return
            active.add(node)
            for dep in graph.get(node, []):
                visit(dep)
            active.remove(node)
            complete.add(node)

        for node in graph:
            visit(node)

    def portable(path):
        parts = PurePosixPath(path).parts
        if path.startswith("/") or ".." in parts or ":" in path or "\\" in path:
            errors.add("paths")
        for part in parts:
            if part.endswith((" ", ".")) or re.match(r"^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)", part, re.I):
                errors.add("paths")

    if kind == "build":
        if len({name.casefold() for name in value}) != len(value):
            errors.add("duplicates")
        for name, target in value.items():
            matrix = target.get("matrix", {})
            if "platform" in target and "platform" in matrix:
                errors.add("platform")
            if len(set(matrix) - {"platform"}) > 1:
                errors.add("platform")
            if "dependencies" in target and target["uses"] != "docker/image":
                errors.add("builder")
            if "bindings" in target and target["uses"] != "helm/chart":
                errors.add("builder")
            if "container" in target and target["uses"] not in {"python/app", "go/app", "node/app", "rust/app", "java/app"}:
                errors.add("builder")
            for binding in target.get("bindings", []):
                if binding["from"] not in value or binding["from"] == name:
                    errors.add("references")
            destinations = []
            for mapping in target.get("materialize", []):
                if mapping["from"] not in value or mapping["from"] == name:
                    errors.add("references")
                portable(mapping["to"])
                path = PurePosixPath(mapping["to"].casefold())
                if any(path == old or path in old.parents or old in path.parents for old in destinations):
                    errors.add("paths")
                destinations.append(path)
            for dependency in target.get("depends_on", []):
                if dependency not in value:
                    errors.add("references")
        graph = {name: target.get("depends_on", []) + [m["from"] for m in target.get("materialize", [])] + [b["from"] for b in target.get("bindings", [])] for name, target in value.items()}
        cycle(graph)
    if kind in {"plan", "manifest"}:
        targets = indexed(value["targets"])
        actions = indexed(value["actions"])
        artifacts = indexed(value["artifacts"])
        for action in actions.values():
            if action["target"] not in targets:
                errors.add("references")
        paths = set()
        for artifact in artifacts.values():
            producer = actions.get(artifact["producer"])
            if artifact["target"] not in targets or producer is None or producer["target"] != artifact["target"]:
                errors.add("references")
            portable(artifact["path"])
            if kind == 'manifest' and artifact['kind'] == 'directory':
                entries = artifact['entries']
                names = [entry['path'] for entry in entries]
                if names != sorted(names) or len({name.casefold() for name in names}) != len(names):
                    errors.add('paths')
                for entry in entries:
                    portable(entry['path'])
                if sum(entry['size'] for entry in entries) != artifact['size']:
                    errors.add('artifacts')
            if artifact["path"].casefold() in paths:
                errors.add("paths")
            paths.add(artifact["path"].casefold())
        if kind == "plan":
            tools = indexed(value["tools"])
            for action in actions.values():
                if any(dep not in actions for dep in action["dependsOn"]):
                    errors.add("references")
                if any(tool not in tools for tool in action["tools"]):
                    errors.add("references")
                for output in action["outputs"]:
                    if output not in artifacts or artifacts[output]["producer"] != action["id"]:
                        errors.add("references")
                for item in action["inputs"]:
                    if item["kind"] == "artifact":
                        if item["artifact"] not in artifacts or item["producer"] not in action["dependsOn"]:
                            errors.add("references")
                        elif artifacts[item["artifact"]]["producer"] != item["producer"]:
                            errors.add("references")
                portable(action["cwd"])
            cycle({key: action["dependsOn"] for key, action in actions.items()})
        else:
            if (value["planDigest"] is None) != (value["planPath"] is None):
                errors.add("references")
            if value["status"] == "succeeded" and (value["planDigest"] is None or value["source"] is None):
                errors.add("status")
            if value["planDigest"] is None and (value["artifacts"] or value["actions"]):
                errors.add("references")
            evidence = indexed(value["evidence"])
            for action in actions.values():
                if any(item not in evidence for item in action["producerEvidence"]):
                    errors.add("references")
                if value["status"] == "succeeded" and action.get("required", True) and action["status"] not in {"succeeded", "cached"}:
                    errors.add("status")
                if action["status"] == "succeeded" and action.get("exitCode", 0) != 0:
                    errors.add("status")
            for report in indexed(value["reports"]).values():
                if report["action"] not in actions or report["target"] not in targets:
                    errors.add("references")
                if report["status"] == "collected" and not {"path", "digest"}.issubset(report):
                    errors.add("reports")
                if "path" in report:
                    portable(report["path"])
                summary = report["summary"]
                if "covered" in summary and ("total" not in summary or summary["covered"] > summary["total"]):
                    errors.add("reports")
    if kind == "dependencies":
        packages = indexed(value["packages"])
        if any(dep not in packages for item in packages.values() for dep in item["dependencies"]):
            errors.add("references")
        # Native package graphs may contain legitimate cycles; do not reject universally.
    if kind == "receipt":
        operations = indexed(value["operations"])
        if value["status"] == "completed" and any(item["state"] != "completed" for item in operations.values()):
            errors.add("status")
    return errors


def main():
    validators = {}
    for file in sorted(CONTRACTS.glob("*.schema.json")):
        schema = read(file)
        Draft202012Validator.check_schema(schema)
        # This suite permits only in-document refs; accidental remote refs fail closed.
        def check_refs(node):
            if isinstance(node, dict):
                if "$ref" in node and not node["$ref"].startswith("#/"):
                    raise ValueError("Nonlocal schema reference: " + node["$ref"])
                for child in node.values():
                    check_refs(child)
            elif isinstance(node, list):
                for child in node:
                    check_refs(child)
        check_refs(schema)
        validators[file.name.removesuffix(".schema.json")] = Draft202012Validator(schema, format_checker=FormatChecker())
    cases = read(CONTRACTS / "fixtures/cases.json")
    failures = []
    for case in cases:
        value = read(CONTRACTS / "fixtures" / case["file"])
        errors = list(validators[case["schema"]].iter_errors(value))
        reasons = {"schema"} if errors else semantics(case["schema"], value)
        if case["valid"] and reasons:
            failures.append(f"{case['file']}: unexpectedly rejected: {reasons}; " + "; ".join(e.message for e in errors[:2]))
        if not case["valid"] and case["reason"] not in reasons:
            failures.append(f"{case['file']}: expected {case['reason']}, observed {reasons}")
    build_examples = sorted((ROOT / "examples").rglob("build.yaml"))
    build_examples += sorted((ROOT / "examples").rglob("*.build.yaml"))
    for path in build_examples:
        value = yaml.safe_load(path.read_text(encoding="utf-8"))
        errors = list(validators["build"].iter_errors(value))
        reasons = {"schema"} if errors else semantics("build", value)
        if reasons:
            failures.append(f"{path.relative_to(ROOT)}: {reasons}; " + "; ".join(e.message for e in errors[:2]))
    if failures:
        raise SystemExit("\n".join(failures))
    print(f"Draft contract checks passed: {len(validators)} schemas, {len(cases)} fixtures, {len(build_examples)} build YAML examples.")
    print("Shape and selected invariants only; no build, credential, digest-content or signature verification.")


if __name__ == "__main__":
    main()
