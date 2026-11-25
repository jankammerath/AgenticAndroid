#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

const dir = path.join(__dirname, "..", "rust-core", "web", "i18n");
const base = JSON.parse(fs.readFileSync(path.join(dir, "en.json"), "utf8"));
const baseKeys = Object.keys(base).sort();

let ok = true;
for (const file of fs.readdirSync(dir)) {
  if (!file.endsWith(".json") || file === "en.json") continue;
  const data = JSON.parse(fs.readFileSync(path.join(dir, file), "utf8"));
  const keys = Object.keys(data).sort();
  const missing = baseKeys.filter((k) => !(k in data));
  const extra = keys.filter((k) => !(k in base));
  if (missing.length || extra.length) {
    ok = false;
    console.error(`[i18n] ${file} mismatch:`);
    if (missing.length) console.error("  Missing:", missing.join(", "));
    if (extra.length) console.error("  Extra:", extra.join(", "));
  } else {
    console.log(`[i18n] ${file}: OK (${keys.length} keys)`);
  }
}

if (!ok) process.exit(1);
console.log(`[i18n] All locale files match en.json (${baseKeys.length} keys).`);
