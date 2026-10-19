// Findings M4 (suffix-tag refusal), B1 (registry-existence refusal) and M3
// (empty-attestations assert): the publish-gate and post-publish legs,
// executed directly with a stub npm registry command.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import { mkTmp, NPM_DIR } from "./helpers.mjs";

const GATE_TAG = path.join(NPM_DIR, "scripts/gate-tag.sh");
const GATE_REGISTRY = path.join(NPM_DIR, "scripts/gate-registry.sh");
const ASSERT_ATT = path.join(NPM_DIR, "scripts/assert-attestations.sh");

function stubNpm(dir, behavior) {
  const file = path.join(dir, `stub-npm-${Math.random().toString(36).slice(2)}.sh`);
  fs.writeFileSync(file, `#!/bin/sh\n${behavior}\n`);
  fs.chmodSync(file, 0o755);
  return file;
}

test("tag-shape gate: exact semver tags pass, suffix tags are refused", () => {
  for (const tag of ["v0.5.2", "v1.0.0", "v10.20.30"]) {
    const res = spawnSync("sh", [GATE_TAG, tag], { encoding: "utf8" });
    assert.equal(res.status, 0, `${tag} must pass: ${res.stderr}`);
  }
  for (const tag of ["v0.5.2-gap", "v1.2", "v1.2.3.4", "v1.2.3+build", "ver1.2.3", "1.2.3"]) {
    const res = spawnSync("sh", [GATE_TAG, tag], { encoding: "utf8" });
    assert.notEqual(res.status, 0, `${tag} must be refused`);
    assert.match(res.stderr, /tag shape/);
  }
});

test("registry-existence gate: existing version refused, absent allowed, unreadable refused", () => {
  const dir = mkTmp("npm-gate-");
  // Stub registry answers 0 (exists), 1 (not found) or 2 (unreadable).
  const exists = stubNpm(dir, "exit 0");
  const absent = stubNpm(dir, "exit 1");
  const broken = stubNpm(dir, "echo 'registry unreachable' >&2; exit 2");
  const common = ["--pkg", "@fixture/archspec", "--version", "1.2.3"];

  const refusal = spawnSync("sh", [GATE_REGISTRY, ...common, "--npm", exists], { encoding: "utf8" });
  assert.notEqual(refusal.status, 0, "an existing <pkg>@<ver> must refuse the run");
  assert.match(refusal.stderr, /republish-freeze/);

  const allow = spawnSync("sh", [GATE_REGISTRY, ...common, "--npm", absent], { encoding: "utf8" });
  assert.equal(allow.status, 0, `an absent version proceeds: ${allow.stderr}`);

  const blind = spawnSync("sh", [GATE_REGISTRY, ...common, "--npm", broken], { encoding: "utf8" });
  assert.notEqual(blind.status, 0, "an unreadable registry fails closed, never green");
});

test("attestations assert: empty dist.attestations is red, non-empty is green", () => {
  const dir = mkTmp("npm-attest-");
  const empty = stubNpm(dir, "exit 0"); // view succeeds, prints nothing
  const present = stubNpm(dir, "echo '-----BEGIN PROVENANCE-----'");
  const common = ["--pkg", "@fixture/archspec", "--version", "1.2.3", "--attempts", "2", "--sleep", "0"];

  const red = spawnSync("sh", [ASSERT_ATT, ...common, "--npm", empty], { encoding: "utf8" });
  assert.notEqual(red.status, 0, "empty attestations must fail the job");
  assert.match(red.stderr, /dist\.attestations/);

  const green = spawnSync("sh", [ASSERT_ATT, ...common, "--npm", present], { encoding: "utf8" });
  assert.equal(green.status, 0, green.stderr);
});
