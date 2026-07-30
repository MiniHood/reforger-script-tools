import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";

const bridgeDirectory = resolve("server/bridge");
const sourceFiles = (await readdir(bridgeDirectory))
  .filter((name) => name.endsWith(".c"))
  .sort();
const failures = [];

for (const sourceFile of sourceFiles) {
  const source = await readFile(resolve(bridgeDirectory, sourceFile), "utf8");
  const lines = source.split("\n");

  if (!source.endsWith("\n")) {
    failures.push(`${sourceFile}: missing final newline`);
  }

  for (let index = 0; index < lines.length; index += 1) {
    const line = lines[index];
    const lineNumber = index + 1;
    if (/^ +\S/.test(line)) {
      failures.push(`${sourceFile}:${lineNumber}: indentation must use tabs`);
    }
    if (/[ \t]+$/.test(line)) {
      failures.push(`${sourceFile}:${lineNumber}: trailing whitespace`);
    }
    assertSingleStatement(sourceFile, lineNumber, line);
    assertControlBody(sourceFile, lines, index);
  }
}

if (failures.length > 0) {
  console.error("Workbench bridge style violations:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exitCode = 1;
} else {
  console.log(`Workbench bridge style verified for ${sourceFiles.length} files.`);
}

function assertSingleStatement(sourceFile, lineNumber, line) {
  let parenDepth = 0;
  let quoted = false;
  let escaped = false;
  let statements = 0;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    const next = line[index + 1];
    if (!quoted && character === "/" && next === "/") {
      break;
    }
    if (character === '"' && !escaped) {
      quoted = !quoted;
    }
    escaped = quoted && character === "\\" && !escaped;
    if (quoted) {
      continue;
    }
    if (character === "(") {
      parenDepth += 1;
    } else if (character === ")") {
      parenDepth -= 1;
    } else if (character === ";" && parenDepth === 0) {
      statements += 1;
    }
  }
  if (statements > 1) {
    failures.push(`${sourceFile}:${lineNumber}: multiple executable statements`);
  }
}

function assertControlBody(sourceFile, lines, index) {
  const line = lines[index];
  const match = line.match(/^(\t*)((?:else\s+)?if|for|foreach|while)\b/);
  if (!match) {
    return;
  }
  const open = line.indexOf("(", match[0].length);
  const close = open < 0 ? -1 : closingParenthesis(line, open);
  if (close < 0) {
    return;
  }
  const remainder = line.slice(close + 1).trim();
  if (remainder && !remainder.startsWith("{") && !remainder.startsWith("//")) {
    failures.push(`${sourceFile}:${index + 1}: control body must begin on the next line`);
    return;
  }
  if (remainder.startsWith("{")) {
    return;
  }
  const next = nextCodeLine(lines, index + 1);
  if (!next) {
    failures.push(`${sourceFile}:${index + 1}: missing control body`);
    return;
  }
  const nextText = lines[next].trimStart();
  if (["for", "foreach", "while"].includes(match[2]) && nextText !== "{") {
    failures.push(`${sourceFile}:${index + 1}: loops require an Allman braced body`);
    return;
  }
  if (match[2].includes("if") && nextText !== "{") {
    const currentIndent = match[1].length;
    const nextIndent = lines[next].length - lines[next].trimStart().length;
    if (nextIndent <= currentIndent) {
      failures.push(`${sourceFile}:${index + 1}: unbraced if body must be indented on the next line`);
    }
  }
}

function closingParenthesis(line, open) {
  let depth = 0;
  let quoted = false;
  let escaped = false;
  for (let index = open; index < line.length; index += 1) {
    const character = line[index];
    if (character === '"' && !escaped) {
      quoted = !quoted;
    }
    escaped = quoted && character === "\\" && !escaped;
    if (quoted) {
      continue;
    }
    if (character === "(") {
      depth += 1;
    } else if (character === ")") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  return -1;
}

function nextCodeLine(lines, start) {
  for (let index = start; index < lines.length; index += 1) {
    if (lines[index].trim() && !lines[index].trimStart().startsWith("//")) {
      return index;
    }
  }
  return undefined;
}
