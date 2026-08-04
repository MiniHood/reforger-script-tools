import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import {
  expectedAgentSkillFiles,
  validateAgentSkills,
} from "./agent-skills.mjs";

test("the repository-owned Agent Skills library satisfies its public package contract", () => {
  const result = validateAgentSkills({ repositoryRoot: process.cwd() });

  assert.deepEqual(result.errors, []);
  assert.deepEqual(result.files, expectedAgentSkillFiles);
  assert.deepEqual(result.skillNames, [
    "reforger",
    "reforger-deep-dive",
    "reforger-workbench-edit",
  ]);
});

test("distributed workflows expose disabled-Workbench and mutation safety gates", () => {
  const general = readFileSync(join("skills", "reforger", "SKILL.md"), "utf8");
  const deepDive = readFileSync(join("skills", "reforger-deep-dive", "SKILL.md"), "utf8");
  const edit = readFileSync(join("skills", "reforger-workbench-edit", "SKILL.md"), "utf8");

  for (const [name, text] of [["reforger", general], ["reforger-deep-dive", deepDive], ["reforger-workbench-edit", edit]]) {
    assert.match(text, /Workbench integration is disabled/i, name);
    assert.match(text, /do not (?:enable it, )?(?:install|perform|send)|perform no Workbench traffic/i, name);
  }
  for (const evidence of ["parser checks", "native compiler outcome", "reload outcome", "live observations"]) {
    assert.ok(general.includes(evidence), `general workflow must distinguish ${evidence}`);
  }
  assert.match(deepDive, /read-only investigation/i);
  assert.match(deepDive, /leave implementation/i);
  for (const gate of ["target", "confirmation", "persist", "read back", "recovery"]) {
    assert.match(edit, new RegExp(gate, "i"), `edit workflow must preserve ${gate}`);
  }
  assert.match(edit, /one mutation at a time/i);
  assert.match(edit, /stop the transaction/i);
});

test("format validation rejects malformed frontmatter, names, descriptions, and instructions", () => {
  withFixture(root => {
    const skill = join(root, "skills", "reforger", "SKILL.md");
    writeFileSync(skill, readFileSync(skill, "utf8")
      .replace("name: reforger", "name: Reforger Skill")
      .replace("description: Ground", `description: ${"x".repeat(1025)}\nignored: Ground`)
      .replace("# Reforger", "plain instructions"));

    const result = validateAgentSkills({ repositoryRoot: root });
    assert.ok(result.errors.some(error => error.includes("frontmatter must contain only name and description")));
    assert.ok(result.errors.some(error => error.includes("name must be 1-64")));
    assert.ok(result.errors.some(error => error.includes("description must be a non-empty string of at most 1024")));
    assert.ok(result.errors.some(error => error.includes("instructions must be parseable")));
  });
});

test("reference validation rejects escapes, duplicates, missing targets, and unlisted files", () => {
  withFixture(root => {
    const skill = join(root, "skills", "reforger", "SKILL.md");
    writeFileSync(skill, `${readFileSync(skill, "utf8")}\n[escape](../../outside.md)\n[duplicate](references/mcp-router.md)\n`);
    rmSync(join(root, "skills", "reforger", "references", "wiki-routes.md"));
    writeFileSync(join(root, "skills", "reforger", "references", "unlisted.md"), "# Unlisted\n");

    const result = validateAgentSkills({ repositoryRoot: root });
    assert.ok(result.errors.some(error => error.includes("escapes the library")));
    assert.ok(result.errors.some(error => error.includes("Missing Agent Skill reference") && error.includes("wiki-routes.md")));
    assert.ok(result.errors.some(error => error.includes("Duplicate Agent Skill reference")));
    assert.ok(result.errors.some(error => error.includes("Unlisted Agent Skill file") && error.includes("unlisted.md")));
  });
});

test("manifest and MCP contract validation reject contribution drift, unknown tools, and tool-scoped field drift", () => {
  withFixture(root => {
    const packagePath = join(root, "package.json");
    const packageJson = JSON.parse(readFileSync(packagePath, "utf8"));
    packageJson.contributes.chatSkills.pop();
    writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

    const router = join(root, "skills", "reforger", "references", "mcp-router.md");
    writeFileSync(router, readFileSync(router, "utf8")
      .replace("`workbench_reload`", "`workbench_reload_typo`")
      .replace("`workbench_validate_scripts.success`", "`workbench_status.success`"));

    const result = validateAgentSkills({ repositoryRoot: root });
    assert.ok(result.errors.some(error => error.includes("contribute exactly the three")));
    assert.ok(result.errors.some(error => error.includes("absent from the generated catalogue: workbench_reload_typo")));
    assert.ok(result.errors.some(error => error.includes("omits material field workbench_status.success")));
  });
});

test("client-neutral validation rejects product syntax and product-specific policy", () => {
  withFixture(root => {
    const skill = join(root, "skills", "reforger", "SKILL.md");
    writeFileSync(skill, `${readFileSync(skill, "utf8")}\nUse $reforger-deep-dive in Claude Code with an approval policy.\n`);
    const result = validateAgentSkills({ repositoryRoot: root });
    assert.ok(result.errors.some(error => error.includes("must remain client-neutral")));
  });
});

function withFixture(run) {
  const root = mkdtempSync(join(tmpdir(), "reforger-agent-skills-"));
  try {
    cpSync("package.json", join(root, "package.json"));
    cpSync("skills", join(root, "skills"), { recursive: true });
    cpSync(join("docs", "mcp-api", "tools"), join(root, "docs", "mcp-api", "tools"), { recursive: true });
    run(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}
