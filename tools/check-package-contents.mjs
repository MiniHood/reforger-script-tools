#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { unzipSync } from "fflate";

const platformArch = `${process.platform}-${process.arch}`;
const vsce = resolve("node_modules", "@vscode", "vsce", "vsce");
const verificationDirectory = resolve(".cache", "package-verification");
const vsixPath = resolve(verificationDirectory, "reforger-script-tools.vsix");
mkdirSync(verificationDirectory, { recursive: true });
const result = spawnSync(process.execPath, [vsce, "package", "--no-dependencies", "--out", vsixPath], {
  encoding: "utf8",
});

if (result.error) {
  console.error(`Could not inspect VSIX contents: ${result.error.message}`);
  process.exit(1);
}
if ((result.status ?? 1) !== 0) {
  process.stderr.write(result.stderr);
  process.exit(result.status ?? 1);
}

const allowedFiles = new Set([
  "extension.vsixmanifest",
  "readme.md",
  "changelog.md",
  "package.json",
  "language-configuration.json",
  "images/icon.jpg",
  "dist/extension.js",
  `dist/server/${platformArch}/reforger_language_server${process.platform === "win32" ? ".exe" : ""}`,
]);
const officialWikiFiles = new Set(listMarkdownFiles(resolve("data", "official-wiki")).map(
  (file) => `data/official-wiki/${file}`,
));
if (!existsSync(vsixPath)) {
  console.error("VSCE completed without creating the verification VSIX.");
  process.exit(1);
}

const files = Object.keys(unzipSync(readFileSync(vsixPath)))
  .map((file) => file.replace(/^extension\//, ""))
  .filter((file) => file !== "extension/" && file !== "[Content_Types].xml");
const unexpected = files.filter(
  (file) => !allowedFiles.has(file) && !officialWikiFiles.has(file),
);
const missingWiki = [...officialWikiFiles].filter((file) => !files.includes(file));

if (unexpected.length > 0 || missingWiki.length > 0) {
  console.error([
    unexpected.length > 0 ? `VSIX contains unexpected files:\n${unexpected.join("\n")}` : "",
    missingWiki.length > 0 ? `VSIX is missing expected Official Wiki files:\n${missingWiki.join("\n")}` : "",
  ].filter(Boolean).join("\n"));
  process.exit(1);
}

console.log(`VSIX allowlist verified (${files.length} files).`);

function listMarkdownFiles(directory, prefix = "") {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const relativePath = `${prefix}${entry.name}`;
    if (entry.isDirectory()) {
      return listMarkdownFiles(resolve(directory, entry.name), `${relativePath}/`);
    }
    return entry.isFile() && entry.name.endsWith(".md") ? [relativePath] : [];
  });
}
