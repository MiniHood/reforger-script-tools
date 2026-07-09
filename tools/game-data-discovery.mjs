#!/usr/bin/env node

import { promises as fs } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const DEFAULT_RELATIVE_SCRIPTS_PATH = path.join(
  'Code',
  'User',
  'globalStorage',
  'undefined_publisher.reforger-sript-tools',
  'game-data',
  'scripts',
);

const DEFAULT_REPORT_PATH = path.join('tools', 'reports', 'game-data-discovery.report.md');
const EXAMPLE_LIMIT = 12;
const LARGEST_FILE_LIMIT = 15;

const CONTROL_KEYWORDS = new Set([
  'if',
  'for',
  'foreach',
  'while',
  'switch',
  'return',
  'else',
  'case',
  'catch',
  'new',
]);

function printHelp() {
  console.log(`Usage:
  node tools/game-data-discovery.mjs
  node tools/game-data-discovery.mjs --scripts <path>
  node tools/game-data-discovery.mjs --scripts <path> --out <path>

Options:
  --scripts <path>  Folder containing Reforger .c scripts.
  --out <path>      Markdown report output path.
  --help            Show this help.`);
}

function parseArgs(argv) {
  const args = {
    scripts: undefined,
    out: DEFAULT_REPORT_PATH,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === '--help' || arg === '-h') {
      args.help = true;
      continue;
    }

    if (arg === '--scripts') {
      args.scripts = argv[index + 1];
      index += 1;
      continue;
    }

    if (arg === '--out') {
      args.out = argv[index + 1];
      index += 1;
      continue;
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  if (!args.scripts) {
    const appData = process.env.APPDATA;
    if (!appData) {
      throw new Error('APPDATA is not set. Pass --scripts <path> explicitly.');
    }

    args.scripts = path.join(appData, DEFAULT_RELATIVE_SCRIPTS_PATH);
  }

  if (!args.out) {
    throw new Error('--out requires a path.');
  }

  return {
    scriptsPath: path.resolve(args.scripts),
    outPath: path.resolve(args.out),
    help: args.help,
  };
}

async function pathExists(filePath) {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}

async function assertScriptsFolder(scriptsPath) {
  let stat;
  try {
    stat = await fs.stat(scriptsPath);
  } catch {
    throw new Error(`Scripts folder was not found: ${scriptsPath}`);
  }

  if (!stat.isDirectory()) {
    throw new Error(`Scripts path is not a directory: ${scriptsPath}`);
  }
}

async function walkScriptFiles(rootPath) {
  const files = [];

  async function visit(currentPath) {
    const entries = await fs.readdir(currentPath, { withFileTypes: true });

    for (const entry of entries) {
      const entryPath = path.join(currentPath, entry.name);

      if (entry.isDirectory()) {
        await visit(entryPath);
        continue;
      }

      if (entry.isFile() && entry.name.toLowerCase().endsWith('.c')) {
        const stat = await fs.stat(entryPath);
        files.push({
          absolutePath: entryPath,
          relativePath: normalizePath(path.relative(rootPath, entryPath)),
          bytes: stat.size,
        });
      }
    }
  }

  await visit(rootPath);
  files.sort((left, right) => left.relativePath.localeCompare(right.relativePath));
  return files;
}

async function readMetadata(scriptsPath) {
  const candidates = [
    path.join(path.dirname(scriptsPath), 'metadata.json'),
    path.join(path.dirname(path.dirname(scriptsPath)), 'metadata.json'),
  ];

  for (const candidate of candidates) {
    if (!(await pathExists(candidate))) {
      continue;
    }

    try {
      const raw = await fs.readFile(candidate, 'utf8');
      return {
        path: candidate,
        data: JSON.parse(raw),
      };
    } catch (error) {
      return {
        path: candidate,
        error: error instanceof Error ? error.message : String(error),
      };
    }
  }

  return undefined;
}

function createAccumulator() {
  return {
    totalLines: 0,
    commentLines: 0,
    blockCommentStarts: 0,
    genericTypeLines: 0,
    inheritedClasses: 0,
    classes: { count: 0, examples: [] },
    moddedClasses: { count: 0, examples: [] },
    enums: { count: 0, examples: [] },
    interfaces: { count: 0, examples: [] },
    methods: { count: 0, examples: [] },
    attributes: { count: 0, examples: [] },
    rpcRpl: { count: 0, examples: [] },
    componentClasses: { count: 0, examples: [] },
    entityClasses: { count: 0, examples: [] },
    workbenchPluginClasses: { count: 0, examples: [] },
    genericExamples: [],
    inheritanceExamples: [],
    commentExamples: [],
  };
}

function addExample(target, file, lineNumber, line) {
  if (target.examples.length >= EXAMPLE_LIMIT) {
    return;
  }

  target.examples.push(formatExample(file.relativePath, lineNumber, line));
}

function addRawExample(target, file, lineNumber, line) {
  if (target.length >= EXAMPLE_LIMIT) {
    return;
  }

  target.push(formatExample(file.relativePath, lineNumber, line));
}

function formatExample(relativePath, lineNumber, line) {
  return {
    location: `${relativePath}:${lineNumber}`,
    snippet: line.trim().replace(/\s+/g, ' '),
  };
}

function scanLine(accumulator, file, line, lineNumber) {
  const trimmed = line.trim();
  const classMatch = trimmed.match(/^(modded\s+)?class\s+([A-Za-z_]\w*)(?:\s*:\s*([A-Za-z_]\w*(?:\s*<[^>]+>)?))?/);
  const enumMatch = trimmed.match(/^enum\s+([A-Za-z_]\w*)/);
  const interfaceMatch = trimmed.match(/^interface\s+([A-Z_]\w*)/);
  const attributeMatch = trimmed.match(/^\[[^\]]+\]/);
  const methodMatch = trimmed.match(/^(?:(?:private|protected|static|override|proto|sealed|event|native|owned|ref|autoptr)\s+)*(?:[A-Za-z_]\w*(?:\s*<[^>]+>)?(?:\[\])?|\w+)\s+([A-Za-z_]\w*)\s*\(/);
  const isCommentLine = trimmed.startsWith('//') || trimmed.startsWith('*') || trimmed.startsWith('/*');

  accumulator.totalLines += 1;

  if (trimmed.startsWith('//') || trimmed.startsWith('*')) {
    accumulator.commentLines += 1;
    addRawExample(accumulator.commentExamples, file, lineNumber, line);
  }

  if (trimmed.includes('/*')) {
    accumulator.blockCommentStarts += 1;
    addRawExample(accumulator.commentExamples, file, lineNumber, line);
  }

  if (/\b[A-Za-z_]\w*\s*<[^>\n]+>/.test(trimmed)) {
    accumulator.genericTypeLines += 1;
    addRawExample(accumulator.genericExamples, file, lineNumber, line);
  }

  if (classMatch) {
    const isModded = Boolean(classMatch[1]);
    const className = classMatch[2];
    const baseName = classMatch[3] ?? '';

    accumulator.classes.count += 1;
    addExample(accumulator.classes, file, lineNumber, line);

    if (isModded) {
      accumulator.moddedClasses.count += 1;
      addExample(accumulator.moddedClasses, file, lineNumber, line);
    }

    if (baseName) {
      accumulator.inheritedClasses += 1;
      addRawExample(accumulator.inheritanceExamples, file, lineNumber, line);
    }

    if (className.includes('Component') || baseName.includes('Component')) {
      accumulator.componentClasses.count += 1;
      addExample(accumulator.componentClasses, file, lineNumber, line);
    }

    if (className.includes('Entity') || baseName.includes('Entity')) {
      accumulator.entityClasses.count += 1;
      addExample(accumulator.entityClasses, file, lineNumber, line);
    }

    if (/WorkbenchPlugin|WorldEditorPlugin|LocalizationEditorPlugin/.test(`${className} ${baseName}`)) {
      accumulator.workbenchPluginClasses.count += 1;
      addExample(accumulator.workbenchPluginClasses, file, lineNumber, line);
    }
  }

  if (enumMatch) {
    accumulator.enums.count += 1;
    addExample(accumulator.enums, file, lineNumber, line);
  }

  if (interfaceMatch) {
    accumulator.interfaces.count += 1;
    addExample(accumulator.interfaces, file, lineNumber, line);
  }

  if (attributeMatch) {
    accumulator.attributes.count += 1;
    addExample(accumulator.attributes, file, lineNumber, line);
  }

  if (
    !isCommentLine
    && !trimmed.startsWith('\\')
    && !trimmed.startsWith('[Obsolete')
    && /\[(?:RplProp|Rpc)\b|\bRpc\b|\bRpl[A-Za-z_]\w*/.test(trimmed)
  ) {
    accumulator.rpcRpl.count += 1;
    addExample(accumulator.rpcRpl, file, lineNumber, line);
  }

  if (methodMatch && !CONTROL_KEYWORDS.has(methodMatch[1])) {
    accumulator.methods.count += 1;
    addExample(accumulator.methods, file, lineNumber, line);
  }
}

async function scanFiles(files) {
  const accumulator = createAccumulator();

  for (const file of files) {
    const content = await fs.readFile(file.absolutePath, 'utf8');
    const lines = content.split(/\r?\n/);

    for (let index = 0; index < lines.length; index += 1) {
      scanLine(accumulator, file, lines[index], index + 1);
    }
  }

  return accumulator;
}

function countByTopLevel(files) {
  const counts = new Map();

  for (const file of files) {
    const [topLevel = '(root)'] = file.relativePath.split('/');
    counts.set(topLevel, (counts.get(topLevel) ?? 0) + 1);
  }

  return [...counts.entries()].sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]));
}

function normalizePath(filePath) {
  return filePath.split(path.sep).join('/');
}

function formatBytes(bytes) {
  if (bytes < 1024) {
    return `${bytes} B`;
  }

  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`;
  }

  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

function escapeTable(value) {
  return String(value).replaceAll('|', '\\|').replace(/\r?\n/g, ' ');
}

function renderMetadata(metadata) {
  if (!metadata) {
    return 'No sibling `metadata.json` was found.';
  }

  if (metadata.error) {
    return `Found metadata at \`${metadata.path}\`, but it could not be parsed: ${metadata.error}`;
  }

  const data = metadata.data;
  const lines = [
    `Metadata path: \`${metadata.path}\``,
    '',
    '| Field | Value |',
    '| --- | --- |',
  ];

  const rows = [
    ['Repository', data.repoUrl ?? data.repositoryUrl ?? '(missing)'],
    ['Branch', data.branch ?? '(missing)'],
    ['Commit SHA', data.commitSha ?? data.sha ?? '(missing)'],
    ['Commit date', data.commitDate ?? data.date ?? '(missing)'],
    ['Commit message', data.commitMessage ?? data.message ?? '(missing)'],
    ['Downloaded at', data.downloadedAt ?? '(missing)'],
    ['Recorded file count', data.fileCount ?? '(missing)'],
    ['Recorded byte count', data.byteCount ?? '(missing)'],
  ];

  for (const [name, value] of rows) {
    lines.push(`| ${escapeTable(name)} | ${escapeTable(value)} |`);
  }

  return lines.join('\n');
}

function renderExamples(title, bucket) {
  const lines = [`### ${title}`, '', `Count: ${bucket.count}`, ''];

  if (bucket.examples.length === 0) {
    lines.push('No examples found.');
    return lines.join('\n');
  }

  for (const example of bucket.examples) {
    lines.push(`- \`${example.location}\` - \`${example.snippet}\``);
  }

  return lines.join('\n');
}

function renderRawExamples(title, count, examples) {
  const lines = [`### ${title}`, '', `Count: ${count}`, ''];

  if (examples.length === 0) {
    lines.push('No examples found.');
    return lines.join('\n');
  }

  for (const example of examples) {
    lines.push(`- \`${example.location}\` - \`${example.snippet}\``);
  }

  return lines.join('\n');
}

function renderParserPriorityNotes(scan) {
  const notes = [
    '- Class declarations and inheritance should be parsed early because inherited classes are common and drive symbol, completion, and type-model behavior.',
    '- Method/function signatures should be a parser priority because declarations are high-volume and will feed references, rename, hover, diagnostics, and formatting.',
  ];

  if (scan.genericTypeLines > 0) {
    notes.push('- Generic type syntax must be handled deliberately; the corpus contains generic-looking type usage and nested type expressions that regex discovery cannot validate.');
  }

  if (scan.attributes.count > 0) {
    notes.push('- Attributes need first-class syntax support because they appear throughout declarations and affect editor-facing meaning.');
  }

  if (scan.rpcRpl.count > 0) {
    notes.push('- RPC and replication patterns need targeted model/index handling after parsing, with Workbench validation before treating behavior as compiler truth.');
  }

  if (scan.commentLines > 0 || scan.blockCommentStarts > 0) {
    notes.push('- Comments and trivia should be retained by the future lexer/parser so formatting, documentation hovers, and source-preserving edits remain possible.');
  }

  if (scan.moddedClasses.count > 0) {
    notes.push('- `modded class` syntax must be modeled separately from normal class declarations because it affects how symbols merge with game-data and workspace scripts.');
  }

  notes.push('- These notes are corpus-discovery guidance only. Workbench/compiler behavior remains the source of truth.');

  return notes.join('\n');
}

function renderReport({ scriptsPath, metadata, files, scan, scannedAt }) {
  const totalBytes = files.reduce((sum, file) => sum + file.bytes, 0);
  const byTopLevel = countByTopLevel(files);
  const largestFiles = [...files].sort((left, right) => right.bytes - left.bytes).slice(0, LARGEST_FILE_LIMIT);

  const sections = [
    '# Game Data Discovery Report',
    '',
    '> Discovery-only report generated by `tools/game-data-discovery.mjs`. Regex findings are not compiler truth.',
    '',
    '## Source',
    '',
    `- Source path: \`${scriptsPath}\``,
    `- Scan timestamp: ${scannedAt}`,
    '',
    '## Metadata',
    '',
    renderMetadata(metadata),
    '',
    '## Corpus Summary',
    '',
    '| Metric | Value |',
    '| --- | ---: |',
    `| .c files | ${files.length} |`,
    `| Bytes | ${totalBytes} (${formatBytes(totalBytes)}) |`,
    `| Lines scanned | ${scan.totalLines} |`,
    '',
    '## File Counts by Top-Level Folder',
    '',
    '| Folder | .c files |',
    '| --- | ---: |',
    ...byTopLevel.map(([folder, count]) => `| ${escapeTable(folder)} | ${count} |`),
    '',
    '## Largest Files',
    '',
    '| File | Bytes | Size |',
    '| --- | ---: | ---: |',
    ...largestFiles.map((file) => `| \`${escapeTable(file.relativePath)}\` | ${file.bytes} | ${formatBytes(file.bytes)} |`),
    '',
    '## Declaration Counts and Examples',
    '',
    renderExamples('Classes', scan.classes),
    '',
    renderExamples('Modded Classes', scan.moddedClasses),
    '',
    renderExamples('Enums', scan.enums),
    '',
    renderExamples('Interfaces', scan.interfaces),
    '',
    renderExamples('Methods and Functions', scan.methods),
    '',
    renderExamples('Attributes', scan.attributes),
    '',
    renderExamples('RPC and Rpl Usage', scan.rpcRpl),
    '',
    renderExamples('Component-Like Classes', scan.componentClasses),
    '',
    renderExamples('Entity-Like Classes', scan.entityClasses),
    '',
    renderExamples('Workbench Plugin Classes', scan.workbenchPluginClasses),
    '',
    '## Parser Priority Signals',
    '',
    '| Signal | Count |',
    '| --- | ---: |',
    `| Classes with inheritance | ${scan.inheritedClasses} |`,
    `| Generic-looking type lines | ${scan.genericTypeLines} |`,
    `| Attribute lines | ${scan.attributes.count} |`,
    `| Method/function-looking declarations | ${scan.methods.count} |`,
    `| Line comments | ${scan.commentLines} |`,
    `| Block comment starts | ${scan.blockCommentStarts} |`,
    `| RPC/Rpl-looking lines | ${scan.rpcRpl.count} |`,
    '',
    renderRawExamples('Inheritance Examples', scan.inheritedClasses, scan.inheritanceExamples),
    '',
    renderRawExamples('Generic Type Examples', scan.genericTypeLines, scan.genericExamples),
    '',
    renderRawExamples('Comment and Trivia Examples', scan.commentLines + scan.blockCommentStarts, scan.commentExamples),
    '',
    '## Parser Priority Notes',
    '',
    renderParserPriorityNotes(scan),
    '',
  ];

  return `${sections.join('\n')}\n`;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    printHelp();
    return;
  }

  await assertScriptsFolder(args.scriptsPath);

  const files = await walkScriptFiles(args.scriptsPath);
  if (files.length === 0) {
    throw new Error(`No .c files were found under scripts folder: ${args.scriptsPath}`);
  }

  const metadata = await readMetadata(args.scriptsPath);
  const scan = await scanFiles(files);
  const report = renderReport({
    scriptsPath: args.scriptsPath,
    metadata,
    files,
    scan,
    scannedAt: new Date().toISOString(),
  });

  await fs.mkdir(path.dirname(args.outPath), { recursive: true });
  await fs.writeFile(args.outPath, report, 'utf8');

  console.log(`Wrote game data discovery report: ${args.outPath}`);
  console.log(`Scanned ${files.length} .c files from: ${args.scriptsPath}`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
