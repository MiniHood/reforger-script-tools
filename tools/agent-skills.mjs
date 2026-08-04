import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, relative, resolve, sep } from "node:path";

export const expectedAgentSkillFiles = [
  "skills/reforger-deep-dive/SKILL.md",
  "skills/reforger-workbench-edit/SKILL.md",
  "skills/reforger-workbench-edit/references/operator-routes.md",
  "skills/reforger/SKILL.md",
  "skills/reforger/references/evidence-contract.md",
  "skills/reforger/references/mcp-router.md",
  "skills/reforger/references/wiki-routes.md",
];

export const expectedAgentSkillContributions = [
  { path: "./skills/reforger/SKILL.md" },
  { path: "./skills/reforger-deep-dive/SKILL.md" },
  { path: "./skills/reforger-workbench-edit/SKILL.md" },
];

const nonToolIdentifiers = new Set([
  "game_data_changed",
  "game_data_unavailable",
]);

export function validateAgentSkills({ repositoryRoot }) {
  const root = resolve(repositoryRoot);
  const skillsRoot = resolve(root, "skills");
  const errors = [];
  const files = existsSync(skillsRoot)
    ? listFiles(skillsRoot).map(file => normalize(relative(root, file))).sort()
    : [];
  const expectedFiles = [...expectedAgentSkillFiles].sort();

  for (const file of files.filter(file => !expectedFiles.includes(file))) {
    errors.push(`Unlisted Agent Skill file: ${file}`);
  }
  for (const file of expectedFiles.filter(file => !files.includes(file))) {
    errors.push(`Missing Agent Skill file: ${file}`);
  }

  const packagePath = resolve(root, "package.json");
  let contributions = [];
  if (existsSync(packagePath)) {
    try {
      const packageJson = JSON.parse(readFileSync(packagePath, "utf8"));
      contributions = packageJson.contributes?.chatSkills ?? [];
      if (JSON.stringify(contributions) !== JSON.stringify(expectedAgentSkillContributions)) {
        errors.push("package.json must contribute exactly the three repository-owned Agent Skills.");
      }
    } catch (error) {
      errors.push(`Could not parse package.json: ${error.message}`);
    }
  } else {
    errors.push("Missing package.json.");
  }

  const skillNames = [];
  const allText = [];
  const reachable = new Set();
  const pending = contributions
    .map(contribution => normalize(String(contribution.path ?? "").replace(/^\.\//, "")))
    .filter(Boolean);

  for (const entryPoint of pending) {
    if (!expectedFiles.includes(entryPoint)) {
      errors.push(`Unlisted Agent Skill entry point: ${entryPoint}`);
    }
  }

  while (pending.length > 0) {
    const source = pending.shift();
    if (reachable.has(source) || !expectedFiles.includes(source)) continue;
    reachable.add(source);
    const sourcePath = resolve(root, source);
    if (!existsSync(sourcePath)) continue;
    const text = readFileSync(sourcePath, "utf8");
    allText.push(text);
    const seenTargets = new Set();
    for (const reference of markdownReferences(text)) {
      if (reference.startsWith("#") || /^[a-z][a-z0-9+.-]*:/i.test(reference)) continue;
      const withoutFragment = decodeURIComponent(reference.split("#", 1)[0]);
      if (!withoutFragment) continue;
      const targetPath = resolve(dirname(sourcePath), withoutFragment);
      const relativeToSkills = relative(skillsRoot, targetPath);
      if (relativeToSkills === ".." || relativeToSkills.startsWith(`..${sep}`)) {
        errors.push(`Agent Skill reference escapes the library: ${source} -> ${reference}`);
        continue;
      }
      const target = normalize(relative(root, targetPath));
      if (seenTargets.has(target)) {
        errors.push(`Duplicate Agent Skill reference: ${source} -> ${target}`);
        continue;
      }
      seenTargets.add(target);
      if (!expectedFiles.includes(target)) {
        errors.push(`Unlisted Agent Skill reference: ${source} -> ${target}`);
        continue;
      }
      if (!existsSync(targetPath)) {
        errors.push(`Missing Agent Skill reference: ${source} -> ${target}`);
        continue;
      }
      pending.push(target);
    }
  }

  for (const file of expectedFiles.filter(file => !reachable.has(file))) {
    errors.push(`Packaged Agent Skill file is unreachable from a contributed SKILL.md: ${file}`);
  }

  for (const skillFile of expectedFiles.filter(file => file.endsWith("/SKILL.md"))) {
    const fullPath = resolve(root, skillFile);
    if (!existsSync(fullPath)) continue;
    const text = readFileSync(fullPath, "utf8");
    const parsed = parseSkill(text, skillFile, errors);
    if (!parsed) continue;
    skillNames.push(parsed.name);
    const directoryName = normalize(dirname(skillFile)).split("/").at(-1);
    if (parsed.name !== directoryName) {
      errors.push(`${skillFile} name must match its directory (${directoryName}).`);
    }
  }

  const libraryText = allText.join("\n");
  if (/\b(?:Codex|Claude Code|Copilot|Visual Studio Code|VS Code|Cursor (?:agent|editor|IDE))\b|\$[a-z0-9][a-z0-9-]*\b|(?:^|\s)\/[a-z][a-z0-9-]*\b|\b(?:approval policy|sandbox permissions?)\b/im.test(libraryText)) {
    errors.push("Agent Skills must remain client-neutral and may not depend on product syntax or product-specific policy.");
  }

  const toolDocsRoot = resolve(root, "docs", "mcp-api", "tools");
  const toolDocs = existsSync(toolDocsRoot)
    ? new Set(readdirSync(toolDocsRoot, { withFileTypes: true })
      .filter(entry => entry.isFile() && entry.name.endsWith(".md"))
      .map(entry => entry.name.slice(0, -3)))
    : new Set();
  const namedIdentifiers = new Set(libraryText.match(/\b(?:official_wiki|search|read|game_data|inspect|list|query|workbench)_[a-z0-9_*]+\b/g) ?? []);
  for (const identifier of namedIdentifiers) {
    if (identifier.endsWith("_*") || nonToolIdentifiers.has(identifier)) continue;
    if (!toolDocs.has(identifier)) {
      errors.push(`Agent Skills name an MCP tool absent from the generated catalogue: ${identifier}`);
    }
  }
  const generatedRouter = existsSync(resolve(root, "docs", "mcp-api.md"))
    ? readFileSync(resolve(root, "docs", "mcp-api.md"), "utf8")
    : "";
  const dependencies = [...libraryText.matchAll(/`(MCP|[a-z][a-z0-9_]+)\.([A-Za-z][A-Za-z0-9]*)`/g)];
  if (dependencies.length === 0) {
    errors.push("Agent Skills must declare material MCP fields as tool.field dependencies.");
  }
  for (const [, owner, field] of dependencies) {
    if (owner === "MCP") {
      if (!generatedRouter.includes(`\`${field}\``) && !generatedRouter.includes(`"${field}"`)) {
        errors.push(`Generated MCP router omits material envelope field MCP.${field}.`);
      }
      continue;
    }
    const tool = owner;
    if (!toolDocs.has(tool)) {
      errors.push(`Agent Skills declare a field dependency on an unknown MCP tool: ${tool}.${field}`);
      continue;
    }
    const toolDocPath = resolve(toolDocsRoot, `${tool}.md`);
    const toolDoc = existsSync(toolDocPath) ? readFileSync(toolDocPath, "utf8") : "";
    if (!toolDoc.includes(`"${field}"`)) {
      errors.push(`Generated MCP catalogue omits material field ${tool}.${field}.`);
    }
  }

  return {
    errors: [...new Set(errors)].sort(),
    files,
    skillNames: skillNames.sort(),
  };
}

function parseSkill(text, file, errors) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]+)$/.exec(text);
  if (!match) {
    errors.push(`${file} must contain YAML frontmatter and Markdown instructions.`);
    return undefined;
  }
  const metadata = {};
  for (const [index, line] of match[1].split(/\r?\n/).entries()) {
    const field = /^([A-Za-z][A-Za-z0-9_-]*):(?:\s+(.*))?$/.exec(line);
    if (!field || field[2] === undefined || field[2].trim() === "") {
      errors.push(`${file} frontmatter line ${index + 1} must be a non-empty scalar mapping entry.`);
      return undefined;
    }
    if (Object.hasOwn(metadata, field[1])) {
      errors.push(`${file} frontmatter repeats ${field[1]}.`);
      return undefined;
    }
    metadata[field[1]] = parseYamlScalar(field[2].trim(), file, index + 1, errors);
    if (metadata[field[1]] === undefined) return undefined;
  }
  const keys = Object.keys(metadata).sort();
  if (JSON.stringify(keys) !== JSON.stringify(["description", "name"])) {
    errors.push(`${file} frontmatter must contain only name and description.`);
  }
  if (typeof metadata.name !== "string" || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(metadata.name) || metadata.name.length > 64) {
    errors.push(`${file} name must be 1-64 lowercase letters, digits, or hyphen-separated words.`);
  }
  if (typeof metadata.description !== "string" || metadata.description.trim().length === 0 || metadata.description.length > 1024) {
    errors.push(`${file} description must be a non-empty string of at most 1024 characters.`);
  }
  if (!/^#\s+\S/m.test(match[2]) || match[2].includes("\0")) {
    errors.push(`${file} instructions must be parseable, non-empty Markdown with a heading.`);
  }
  return metadata;
}

function markdownReferences(text) {
  return [...text.matchAll(/\[[^\]]*\]\(([^)\s]+)(?:\s+["'][^"']*["'])?\)/g)].map(match => match[1]);
}

function parseYamlScalar(value, file, line, errors) {
  if (value.startsWith("[") || value.startsWith("{") || /^[|>&*!]/.test(value)) {
    errors.push(`${file} frontmatter line ${line} must use a plain or quoted string scalar.`);
    return undefined;
  }
  if (value.startsWith('"')) {
    try {
      const parsed = JSON.parse(value);
      if (typeof parsed === "string") return parsed;
    } catch {
      // Report the shared scalar error below.
    }
    errors.push(`${file} frontmatter line ${line} contains an invalid quoted string.`);
    return undefined;
  }
  if (value.startsWith("'")) {
    if (!value.endsWith("'") || value.length < 2) {
      errors.push(`${file} frontmatter line ${line} contains an invalid quoted string.`);
      return undefined;
    }
    return value.slice(1, -1).replaceAll("''", "'");
  }
  if (/\s+#/.test(value) || /:\s/.test(value)) {
    errors.push(`${file} frontmatter line ${line} must quote YAML comments or mapping-like text.`);
    return undefined;
  }
  return value;
}

function listFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? listFiles(path) : entry.isFile() ? [path] : [];
  });
}

function normalize(path) {
  return path.replaceAll("\\", "/");
}
