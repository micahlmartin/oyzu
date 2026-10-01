#!/usr/bin/env node
// Dependency-free checks for the Markdown design library, not product behavior.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];
function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    if (entry.name === ".git" || entry.name === "node_modules") return [];
    const file = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(file) : file.endsWith(".md") ? [file] : [];
  });
}
const files = walk(root);
const documents = files.map(file => ({ file, text: fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n") }));
const ids = new Map();
const acceptance = new Set();
const relative = file => path.relative(root, file).split(path.sep).join("/");
const fail = (file, message) => failures.push(relative(file) + ": " + message);
const withoutCode = text => text.replace(/^(\x60{3,}|~{3,})[^\n]*\n[\s\S]*?^\1[ \t]*$/gm, "");

for (const doc of documents) {
  if (/[ \t]+$/m.test(doc.text)) fail(doc.file, "trailing whitespace");
  if (!doc.text.endsWith("\n")) fail(doc.file, "missing final newline");
  const front = doc.text.match(/^---\n([\s\S]*?)\n---\n/);
  if (front) {
    const fields = new Map(front[1].split("\n").map(line => {
      const colon = line.indexOf(":");
      return [line.slice(0, colon), line.slice(colon + 1).trim()];
    }));
    const id = fields.get("id");
    if (!id || !/^(VIS-\d{3}|OEP-(?:E-)?\d{4})$/.test(id)) fail(doc.file, "invalid document ID");
    if (ids.has(id)) fail(doc.file, "duplicate document ID " + id);
    ids.set(id, { ...doc, fields });
    if (!/^\d{4}-\d{2}-\d{2}$/.test(fields.get("updated") ?? "")) fail(doc.file, "missing ISO updated date");
    if (!["draft", "in-review", "accepted", "rejected", "withdrawn", "superseded"].includes(fields.get("status"))) fail(doc.file, "invalid design status");
    if (id?.startsWith("OEP-")) {
      for (const key of ["title", "implementation", "authors", "reviewers", "requires", "tracking-issue"]) {
        if (!fields.has(key)) fail(doc.file, "missing metadata: " + key);
      }
      if (!["not-started", "in-progress", "experimental", "stable"].includes(fields.get("implementation"))) fail(doc.file, "invalid delivery state");
      if (!relative(doc.file).includes("/" + id + "-")) fail(doc.file, "ID differs from directory");
      if (!/^\[.*\]$/.test(fields.get("requires") ?? "")) fail(doc.file, "requires must be an inline ID list");
      if (!/^## (Acceptance|Verification)/m.test(doc.text)) fail(doc.file, "missing acceptance section");
      if (!/^## .*Open|^## .*open/m.test(doc.text)) fail(doc.file, "missing open decisions section");
    }
  } else if (/\/(?:OEP-(?:E-)?\d{4}-[^/]+\/README|VIS-\d{3}-[^/]+)\.md$/.test(relative(doc.file))) {
    fail(doc.file, "missing front matter");
  }
  let fence;
  for (const line of doc.text.split("\n")) {
    const match = line.match(/^(\x60{3,}|~{3,})/);
    if (!match) continue;
    if (!fence) fence = match[1];
    else if (match[1][0] === fence[0] && match[1].length >= fence.length) fence = undefined;
  }
  if (fence) fail(doc.file, "unclosed fenced code block");
  for (const match of doc.text.matchAll(/^- ([A-Z]+(?:-[A-Z]+)*-\d{2}):/gm)) {
    if (acceptance.has(match[1])) fail(doc.file, "duplicate acceptance ID " + match[1]);
    acceptance.add(match[1]);
  }
  for (const match of withoutCode(doc.text).matchAll(/!?\[[^\]\n]*\]\(([^)\n]+)\)/g)) {
    const target = match[1].replace(/^<|>$/g, "");
    if (/^(https?:|mailto:|#)/i.test(target)) continue;
    if (/^[a-z]+:/i.test(target)) { fail(doc.file, "unsupported/absolute link " + target); continue; }
    const local = decodeURIComponent(target.split("#")[0]);
    const destination = path.resolve(path.dirname(doc.file), local);
    if (path.relative(root, destination).startsWith("..") || path.isAbsolute(local)) {
      fail(doc.file, "link escapes repository: " + target);
    } else if (!fs.existsSync(destination)) fail(doc.file, "broken local link: " + target);
  }
}
const graph = new Map();
for (const [id, doc] of ids) {
  const deps = (doc.fields.get("requires") ?? "[]").slice(1, -1).split(",").map(x => x.trim()).filter(Boolean);
  graph.set(id, deps);
  for (const dep of deps) if (!ids.has(dep)) fail(doc.file, "unknown dependency " + dep);
}
const visited = new Set();
function visit(id, ancestors = []) {
  if (ancestors.includes(id)) { failures.push("Dependency cycle: " + [...ancestors, id].join(" -> ")); return; }
  if (visited.has(id)) return;
  for (const dep of graph.get(id) ?? []) visit(dep, [...ancestors, id]);
  visited.add(id);
}
for (const id of ids.keys()) visit(id);
const catalog = documents.find(doc => relative(doc.file) === "docs/examples.md");
if (catalog) {
  const exampleIds = new Set();
  for (const match of catalog.text.matchAll(/^\| (EX-\d{3}) \|/gm)) {
    if (exampleIds.has(match[1])) fail(catalog.file, "duplicate example " + match[1]);
    exampleIds.add(match[1]);
  }
  for (const match of catalog.text.matchAll(/\b(?:DOC|CFG|MISE|TOOL|TASK|PLAN|EXEC|ENV|AGENT|CONN|CACHE|BUNDLE|REL|BUILDER|DIST|PROTO)-\d{2}\b/g)) {
    if (!acceptance.has(match[0])) fail(catalog.file, "unknown acceptance reference " + match[0]);
  }
}
if (failures.length) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Documentation checks passed: " + files.length + " Markdown files, " + ids.size + " design IDs, " + acceptance.size + " acceptance criteria.");
}
