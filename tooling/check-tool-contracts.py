"""Check shared OEP-0003 shape fixtures, not installation or backend acceptance."""
import copy
import json
from pathlib import Path

try:
    from jsonschema import Draft202012Validator
except ImportError:
    raise SystemExit("Install tooling/design-requirements.txt in the validation environment.")

ROOT = Path(__file__).resolve().parents[1]


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key: " + key)
        result[key] = value
    return result


def read(path):
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)


def main():
    schema = read(ROOT / "docs/contracts/tools-v1/archive-layout.schema.json")
    # All references are local fragments. Never resolve remote schema resources.
    def local(value):
        if isinstance(value, dict):
            if "$ref" in value and not value["$ref"].startswith("#/"):
                raise ValueError("nonlocal schema reference")
            for child in value.values():
                local(child)
        elif isinstance(value, list):
            for child in value:
                local(child)
    local(schema)
    Draft202012Validator.check_schema(schema)
    validator = Draft202012Validator(schema)
    fixture = read(ROOT / "tests/fixtures/tool-layout/plan.json")
    validator.validate(fixture)
    cases = read(ROOT / "tests/fixtures/tool-layout/invalid-shapes.json")
    for case in cases:
        changed = copy.deepcopy(fixture)
        parts = case["pointer"].split("/")[1:]
        parent = changed
        for part in parts[:-1]:
            parent = parent[int(part)] if isinstance(parent, list) else parent[part]
        if case.get("remove"):
            del parent[parts[-1]]
        else:
            parent[parts[-1]] = case["value"]
        if validator.is_valid(changed):
            raise AssertionError("schema accepted invalid case: " + case["name"])
    print(f"Tool layout shape checks passed: 1 valid and {len(cases)} invalid fixtures; no backend qualification.")


if __name__ == "__main__":
    main()
