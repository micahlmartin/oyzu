"""Validate example design files; does not execute an Oyzu implementation.

Requires Python 3.11+ and PyYAML 6.x for YAML syntax checks.
"""
import ast
import json
import re
import sys
import tomllib
import xml.etree.ElementTree as ET
from pathlib import Path

try:
    import yaml
except ImportError:
    raise SystemExit("Install PyYAML 6.x in your validation environment; no example project dependency is required.")

ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = ROOT / "examples"
IGNORED = {"node_modules", ".venv", "__pycache__", ".gradle", "target", "dist", ".git", "coverage", ".events"}
errors = []
counts = {"json": 0, "toml": 0, "yaml": 0, "xml": 0, "python": 0}
ids = set()

class UniqueLoader(yaml.SafeLoader):
    pass

def unique_mapping(loader, node, deep=False):
    result = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in result:
            raise ValueError("duplicate YAML key: " + str(key))
        result[key] = loader.construct_object(value_node, deep=deep)
    return result

UniqueLoader.add_constructor(yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, unique_mapping)

def unique_json(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key: " + key)
        result[key] = value
    return result

def contained(base, value):
    path = (base / value).resolve()
    if not path.is_relative_to(base.resolve()):
        raise ValueError("path escapes scenario: " + value)
    return path

criteria = set()
for proposal in (ROOT / "docs/proposals").glob("OEP-*/README.md"):
    criteria.update(re.findall(r"^- ([A-Z]+(?:-[A-Z]+)*-\d{2}):", proposal.read_text(encoding="utf-8"), re.M))
catalog_ids = set(re.findall(r"EX-\d{3}", (ROOT / "docs/examples.md").read_text(encoding="utf-8")))

files = [p for p in EXAMPLES.rglob("*") if p.is_file() and not any(part in IGNORED for part in p.relative_to(EXAMPLES).parts)]
for file in files:
    try:
        text = file.read_text(encoding="utf-8")
        relative = file.relative_to(EXAMPLES)
        if file.suffix == ".json":
            data = json.loads(text, object_pairs_hook=unique_json)
            counts["json"] += 1
            if file.name == "scenario.json":
                if data["id"] in ids:
                    raise ValueError("duplicate scenario ID")
                ids.add(data["id"])
                if data.get("fixtureStatus") != "authored" or data.get("oyzuStatus") != "pending-implementation":
                    raise ValueError("examples must not claim implemented Oyzu behavior")
                if not data["expected"] or not data["cases"] or not data["acceptance"]:
                    raise ValueError("missing expected outcomes, negative cases, or criteria")
                for criterion in data["acceptance"]:
                    if criterion not in criteria:
                        raise ValueError("unknown acceptance criterion: " + criterion)
                for project in data["projects"]:
                    if not contained(file.parent, project).is_dir():
                        raise ValueError("missing project root: " + project)
                for required in data["requiredFiles"]:
                    if not contained(file.parent, required).is_file():
                        raise ValueError("missing required input: " + required)
                for check in data["nativeChecks"]:
                    if not contained(file.parent, check["cwd"]).is_dir():
                        raise ValueError("invalid native command directory")
                if not (file.parent / "README.md").is_file():
                    raise ValueError("missing scenario guide")
        elif file.suffix == ".toml" or file.name in {"Cargo.lock", "poetry.lock", "uv.lock"}:
            tomllib.loads(text)
            counts["toml"] += 1
        elif file.suffix in {".yaml", ".yml"} and "templates" not in relative.parts:
            yaml.load(text, Loader=UniqueLoader)
            counts["yaml"] += 1
        elif file.suffix == ".xml":
            ET.fromstring(text)
            counts["xml"] += 1
        elif file.suffix == ".py":
            ast.parse(text, filename=str(relative))
            counts["python"] += 1
    except (ValueError, KeyError, TypeError, SyntaxError, ET.ParseError, yaml.YAMLError) as exc:
        errors.append(str(file.relative_to(ROOT)) + ": " + str(exc))

if ids != catalog_ids:
    errors.append("catalog/scenario mismatch: " + repr(sorted(ids ^ catalog_ids)))

zero_config = [
    "tasks/go-implicit/project", "builds/python-uv-library/project",
    "builds/go-app/project", "builds/node-package/project",
    "builds/rust-app/project", "builds/java-maven-reactor/project",
]
for project in zero_config:
    for config in ["build.yaml", "oyzu.toml"]:
        if (EXAMPLES / project / config).exists():
            errors.append(project + ": conventional baseline gained " + config)

if errors:
    print("\n".join(errors), file=sys.stderr)
    raise SystemExit(1)
print(f"Example contract checks passed: {len(ids)} scenarios; syntax checks: {counts}.")
print("No Oyzu execution or cross-platform implementation claims were tested.")
