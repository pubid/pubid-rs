#!/usr/bin/env node
// The TS engine of the differential runner. Cases go in as one JSON
// file (--cases path), the verdicts come back as one JSON object per
// line: an engine header first (availability + version), then one
// verdict per case in input order. --ts-root points at a pubid-ts
// checkout or npm tree carrying dist/index.js.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const args = process.argv.slice(2);
const flag = (name) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : undefined;
};

const casesPath = flag("--cases");
if (!casesPath) {
  console.log(JSON.stringify({ status: "engine", available: false, detail: "no --cases path" }));
  process.exit(0);
}
const tsRoot = flag("--ts-root");
if (!tsRoot) {
  console.log(JSON.stringify({ status: "engine", available: false, detail: "no --ts-root" }));
  process.exit(0);
}

let mod;
let version = "unknown";
try {
  mod = await import(pathToFileURL(join(tsRoot, "dist", "index.js")).href);
  version = JSON.parse(readFileSync(join(tsRoot, "package.json"), "utf8")).version ?? "unknown";
} catch (err) {
  console.log(
    JSON.stringify({ status: "engine", available: false, detail: `${err?.code ?? err}` }),
  );
  process.exit(0);
}

console.log(JSON.stringify({ status: "engine", available: true, version }));

const cases = JSON.parse(readFileSync(casesPath, "utf8"));
for (const entry of cases) {
  const impl = mod.grammarImplementation(entry.flavor);
  if (!impl) {
    console.log(JSON.stringify({ i: entry.i, status: "unimplemented" }));
    continue;
  }
  try {
    const id = impl.parse(entry.input);
    console.log(
      JSON.stringify({
        i: entry.i,
        status: "ok",
        hash: id.toHash(),
        human: id.toHuman(),
        urn: id.toUrn() ?? null,
      }),
    );
  } catch (err) {
    console.log(
      JSON.stringify({ i: entry.i, status: "reject", error_code: "parse_failed" }),
    );
  }
}
