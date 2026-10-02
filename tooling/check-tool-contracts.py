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


def check_contract(name, schema_path, fixture_path, cases_path, valid_path=None):
    schema = read(ROOT / schema_path)
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
    fixture = read(ROOT / fixture_path)
    validator.validate(fixture)
    cases = read(ROOT / cases_path)
    valid_cases = read(ROOT / valid_path) if valid_path else []
    for case, expected in [(case, False) for case in cases] + [(case, True) for case in valid_cases]:
        changed = copy.deepcopy(fixture)
        parts = case["pointer"].split("/")[1:]
        parent = changed
        for part in parts[:-1]:
            parent = parent[int(part)] if isinstance(parent, list) else parent[part]
        if case.get("remove"):
            del parent[parts[-1]]
        else:
            parent[parts[-1]] = case["value"]
        if validator.is_valid(changed) != expected:
            raise AssertionError("schema disagrees with case: " + case["name"])
    print(f"Tool {name} shape checks passed: {1 + len(valid_cases)} valid and {len(cases)} invalid fixtures; no backend qualification.")


def main():
    check_contract("request identity", "docs/contracts/tools-v1/request-identity.schema.json",
                   "tests/fixtures/tool-requests/identity.json", "tests/fixtures/tool-requests/invalid-shapes.json")
    check_contract("worker request envelope", "docs/contracts/tools-v1/worker-envelope.schema.json",
                   "tests/fixtures/tool-worker/request.json", "tests/fixtures/tool-worker/invalid-requests.json")
    check_contract("worker response envelope", "docs/contracts/tools-v1/worker-envelope.schema.json",
                   "tests/fixtures/tool-worker/response.json", "tests/fixtures/tool-worker/invalid-responses.json")
    check_contract("selection grant", "docs/contracts/tools-v1/selection-grant.schema.json",
                   "tests/fixtures/tool-grant/payload.json", "tests/fixtures/tool-grant/invalid-shapes.json",
                   "tests/fixtures/tool-grant/valid-shapes.json")
    check_contract("offline selection grant", "docs/contracts/tools-v1/selection-grant.schema.json",
                   "tests/fixtures/tool-grant/offline-payload.json", "tests/fixtures/tool-grant/offline-invalid-shapes.json")
    check_contract("backend descriptor", "docs/contracts/tools-v1/backend-descriptor.schema.json",
                   "tests/fixtures/tool-backend/descriptor.json", "tests/fixtures/tool-backend/invalid-shapes.json",
                   "tests/fixtures/tool-backend/valid-shapes.json")
    check_contract("layout", "docs/contracts/tools-v1/archive-layout.schema.json",
                   "tests/fixtures/tool-layout/plan.json", "tests/fixtures/tool-layout/invalid-shapes.json")
    check_contract("raw layout", "docs/contracts/tools-v1/archive-layout.schema.json",
                   "tests/fixtures/tool-layout/raw-plan.json", "tests/fixtures/tool-layout/raw-invalid-shapes.json")
    check_contract("receipt", "docs/contracts/tools-v1/receipt.schema.json",
                   "tests/fixtures/tool-receipt/receipt.json", "tests/fixtures/tool-receipt/invalid-shapes.json",
                   "tests/fixtures/tool-receipt/valid-shapes.json")


if __name__ == "__main__":
    main()
