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

test("activation metadata distinguishes explicit, implicit, and non-Reforger scenarios", () => {
  const { skills } = validateAgentSkills({ repositoryRoot: process.cwd() });
  const scenarios = [
    { prompt: "Use reforger to implement an Enforce Script component and validate it in Workbench.", expected: "reforger" },
    { prompt: "Implement an Arma Reforger multiplayer component, verify its engine API, compile it, reload, and test it live.", expected: "reforger" },
    { prompt: "Use reforger-deep-dive to investigate this uncertain replication failure without changing anything.", expected: "reforger-deep-dive" },
    { prompt: "Forensically investigate competing root causes for a difficult Arma Reforger failure and produce a read-only evidence dossier.", expected: "reforger-deep-dive" },
    { prompt: "Use reforger-workbench-edit to move this selected World Editor entity and save it with readback.", expected: "reforger-workbench-edit" },
    { prompt: "Move a selected live Arma Reforger Workbench entity after inspection, confirmation, persistence, and readback.", expected: "reforger-workbench-edit" },
    { prompt: "Refactor this generic TypeScript date utility and update its unit tests.", expected: undefined },
  ];

  for (const scenario of scenarios) {
    assert.strictEqual(selectSkill(skills, scenario.prompt), scenario.expected, scenario.prompt);
  }
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

test("manifest and MCP contract validation reject contribution drift, unknown tools, and omitted material fields", () => {
  withFixture(root => {
    const packagePath = join(root, "package.json");
    const packageJson = JSON.parse(readFileSync(packagePath, "utf8"));
    packageJson.contributes.chatSkills.pop();
    writeFileSync(packagePath, `${JSON.stringify(packageJson, null, 2)}\n`);

    const router = join(root, "skills", "reforger", "references", "mcp-router.md");
    writeFileSync(router, readFileSync(router, "utf8")
      .replaceAll("`readInput`", "readInput")
      .replace("`workbench_reload`", "`workbench_reload_typo`"));
    const wikiRoutes = join(root, "skills", "reforger", "references", "wiki-routes.md");
    writeFileSync(wikiRoutes, readFileSync(wikiRoutes, "utf8").replaceAll("`readInput`", "readInput"));

    const result = validateAgentSkills({ repositoryRoot: root });
    assert.ok(result.errors.some(error => error.includes("contribute exactly the three")));
    assert.ok(result.errors.some(error => error.includes("absent from the generated catalogue: workbench_reload_typo")));
    assert.ok(result.errors.some(error => error.includes("omit material field search_official_wiki.readInput")));
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

function selectSkill(skills, prompt) {
  const normalizedPrompt = prompt.toLowerCase();
  const candidates = skills.map(skill => {
    if (normalizedPrompt.includes(`use ${skill.name}`)) return { name: skill.name, score: 100 + skill.name.length };
    const terms = new Set(skill.description.toLowerCase().match(/[a-z][a-z-]{3,}/g) ?? []);
    const promptTerms = new Set(normalizedPrompt.match(/[a-z][a-z-]{3,}/g) ?? []);
    const score = [...promptTerms].filter(term => terms.has(term)).length;
    return { name: skill.name, score };
  }).sort((left, right) => right.score - left.score || left.name.localeCompare(right.name));
  return candidates[0]?.score >= 2 ? candidates[0].name : undefined;
}
