// Parse every sitometres spec against the harness's own schema.
//
// Cheap by construction: it builds no host and no module, so a malformed spec
// is caught in seconds on every PR rather than after a Basecamp build. That
// separation is the whole point — the same mistake found after the expensive
// job is the same information arriving much later.
//
// THE FILE LIST IS GLOBBED, NEVER MAINTAINED BESIDE THE DIRECTORY. A spec
// present in the tree but absent from a hand-kept list runs nowhere, passes
// nothing, and is indistinguishable from a spec that passes. Radicle hit
// exactly that: its `SPEC` was hardcoded to one file while three specs sat in
// the tree running nowhere.
//
// It also FAILS ON AN EMPTY DIRECTORY. A validator that reports success over
// zero files is a gate that cannot fail, which is the defect this repository
// has shipped before and now checks for explicitly.
//
// Run it with:  node dialectica-ui/tests/validate-ui-specs.mjs

import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { parse } from "yaml";
import { validateSpec } from "@paradoxcomputer/sitometres";

const dir = path.join(path.dirname(new URL(import.meta.url).pathname), "ui");

const files = readdirSync(dir)
  .filter((f) => f.endsWith(".yaml") || f.endsWith(".yml"))
  .sort();

if (files.length === 0) {
  console.error(`::error::no specs found in ${dir} — a validator over zero files cannot fail`);
  process.exit(1);
}

let failed = 0;
for (const file of files) {
  const full = path.join(dir, file);
  try {
    // The step count is REPORTED, not merely counted: it is the number the
    // expensive job's adjudicator compares the report against, so seeing it
    // here is what makes a step-count mismatch there readable.
    const spec = validateSpec(parse(readFileSync(full, "utf8")));
    console.log(`${file}: ok (${spec.steps.length} steps)`);
  } catch (err) {
    console.error(`::error file=dialectica-ui/tests/ui/${file}::${err.message}`);
    failed += 1;
  }
}

if (failed > 0) {
  console.error(`${failed} of ${files.length} spec(s) did not parse`);
  process.exit(1);
}
console.log(`ok: ${files.length} spec(s) parsed`);
