#!/usr/bin/env node

import { promises as fs } from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const DEFAULT_SCRIPTS_PATH = path.join(
  process.env.APPDATA ?? '',
  'Code',
  'User',
  'globalStorage',
  'undefined_publisher.reforger-sript-tools',
  'game-data',
  'scripts',
);
const DEFAULT_OUT_PATH = path.join('tools', 'reports', 'comment-formatting-corpus.report.md');
const EXAMPLE_LIMIT = 8;
const TAGS = ['\\brief', '\\param', '@param', '\\return', '@return', '\\warning', '@warning', '\\note', '@note', '\\code', '\\endcode', '\\see', '\\ref'];

function parseArgs(argv) {
  const result = { scripts: DEFAULT_SCRIPTS_PATH, out: DEFAULT_OUT_PATH };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === '--help' || argument === '-h') {
      result.help = true;
    } else if (argument === '--scripts' || argument === '--out') {
      const value = argv[index + 1];
      if (!value) throw new Error(`${argument} requires a path.`);
      result[argument.slice(2)] = value;
      index += 1;
    } else {
      throw new Error(`Unknown argument: ${argument}`);
    }
  }
  return { ...result, scripts: path.resolve(result.scripts), out: path.resolve(result.out) };
}

function printHelp() {
  console.log(`Usage: node tools/comment-formatting-corpus-report.mjs [--scripts <path>] [--out <path>]\n\nThis discovery report counts text patterns only; it is not Workbench/compiler truth.`);
}

async function walkScripts(root) {
  const files = [];
  async function visit(current) {
    for (const entry of await fs.readdir(current, { withFileTypes: true })) {
      const full = path.join(current, entry.name);
      if (entry.isDirectory()) await visit(full);
      else if (entry.isFile() && entry.name.toLowerCase().endsWith('.c')) files.push(full);
    }
  }
  await visit(root);
  return files.sort();
}

function bucket() {
  return { count: 0, examples: [] };
}

function record(target, relativePath, lineNumber, line) {
  target.count += 1;
  if (target.examples.length < EXAMPLE_LIMIT) {
    target.examples.push(`${relativePath}:${lineNumber} - ${line.trim().replace(/\s+/g, ' ')}`);
  }
}

function createScan() {
  return {
    indentation: { tabs: 0, spaces: 0, mixed: 0, none: 0 },
    braces: { allman: 0, inline: 0 },
    comments: Object.fromEntries(['//', '///', '//!', '//!<', '/*', '/*!', '/**'].map((name) => [name, bucket()])),
    tags: Object.fromEntries(TAGS.map((tag) => [tag, bucket()])),
    attributes: bucket(),
    callableParameters: bucket(),
    singleLineControls: bucket(),
    semicolonCandidates: bucket(),
    classifications: { generated: 0, proto: 0, native: 0, workbench: 0, ordinary: 0 },
  };
}

function classify(relativePath, content) {
  const lower = `${relativePath}\n${content.slice(0, 4096)}`.toLowerCase();
  if (lower.includes('generated')) return 'generated';
  if (/\b(proto|native)\b/.test(content)) return /\bproto\b/.test(content) ? 'proto' : 'native';
  if (lower.includes('workbench')) return 'workbench';
  return 'ordinary';
}

function scanLine(scan, relativePath, line, lineNumber, commentState) {
  const trimmed = line.trim();
  if (!trimmed) return;
  const leading = line.match(/^[\t ]*/)?.[0] ?? '';
  if (!leading) scan.indentation.none += 1;
  else if (/^\t+$/.test(leading)) scan.indentation.tabs += 1;
  else if (/^ +$/.test(leading)) scan.indentation.spaces += 1;
  else scan.indentation.mixed += 1;

  if (trimmed === '{') scan.braces.allman += 1;
  else if (/\S\s*\{$/.test(trimmed)) scan.braces.inline += 1;

  const commentForm = trimmed.startsWith('//!<') ? '//!<' : trimmed.startsWith('//!') ? '//!' : trimmed.startsWith('///') ? '///' : trimmed.startsWith('//') ? '//' : trimmed.startsWith('/*!') ? '/*!' : trimmed.startsWith('/**') ? '/**' : trimmed.startsWith('/*') ? '/*' : undefined;
  if (commentForm) record(scan.comments[commentForm], relativePath, lineNumber, line);
  for (const tag of TAGS) {
    if (trimmed.includes(tag)) record(scan.tags[tag], relativePath, lineNumber, line);
  }
  const isBlockStart = !commentState.inBlock && Boolean(commentForm && commentForm.startsWith('/*'));
  const isComment = commentState.inBlock || Boolean(commentForm);
  if (commentState.inBlock && trimmed.includes('*/')) commentState.inBlock = false;
  else if (isBlockStart && !trimmed.includes('*/')) commentState.inBlock = true;
  if (isComment) return;
  if (trimmed.startsWith('[')) record(scan.attributes, relativePath, lineNumber, line);
  const callable = trimmed.match(/^(?:[A-Za-z_]\w*\s+)*(?:[A-Za-z_]\w*(?:<[^>]+>)?)\s+(~?[A-Za-z_]\w*)\s*\(([^)]*)\)/);
  if (callable) {
    record(scan.callableParameters, relativePath, lineNumber, line);
  }

  const codeBeforeLineComment = trimmed.split('//', 1)[0].trimEnd();
  if (!callable && /\)$/.test(codeBeforeLineComment) && !/\b(if|for|foreach|while|switch)\s*\(/.test(codeBeforeLineComment) && !codeBeforeLineComment.endsWith(';')) {
    record(scan.semicolonCandidates, relativePath, lineNumber, line);
  }
}

function scanSingleLineControls(scan, relativePath, lines) {
  for (let index = 0; index < lines.length; index += 1) {
    const header = lines[index].trim();
    if (!/^(if|for|foreach|while)\s*\(.+\)$/.test(header) || header.includes('{')) continue;
    const body = lines.slice(index + 1).find((line) => line.trim());
    if (body && !body.trim().startsWith('{')) record(scan.singleLineControls, relativePath, index + 1, lines[index]);
  }
}

async function scanCorpus(root) {
  const scan = createScan();
  const files = await walkScripts(root);
  for (const absolutePath of files) {
    const content = await fs.readFile(absolutePath, 'utf8');
    const relativePath = path.relative(root, absolutePath).split(path.sep).join('/');
    scan.classifications[classify(relativePath, content)] += 1;
    const lines = content.split(/\r?\n/);
    const commentState = { inBlock: false };
    lines.forEach((line, index) => scanLine(scan, relativePath, line, index + 1, commentState));
    scanSingleLineControls(scan, relativePath, lines);
  }
  return { files, scan };
}

function section(title, entries) {
  const lines = [`## ${title}`, '', '| Shape | Count | Examples |', '| --- | ---: | --- |'];
  for (const [name, value] of Object.entries(entries)) {
    const examples = value.examples?.map((example) => `\`${example.replaceAll('|', '\\|')}\``).join('<br>') ?? '';
    lines.push(`| \`${name}\` | ${value.count ?? value} | ${examples || '-'} |`);
  }
  return lines.join('\n');
}

function render(root, files, scan) {
  return [
    '# Comment Formatting Corpus Report',
    '',
    '> Discovery-only output. Counts are text-pattern signals, not Enfusion grammar, Workbench behavior, or formatter eligibility.',
    '',
    '## Source',
    '',
    `- Scripts: \`${root}\``,
    `- Generated: ${new Date().toISOString()}`,
    `- Files: ${files.length}`,
    '',
    section('Source Classification', scan.classifications),
    '',
    section('Indentation', scan.indentation),
    '',
    section('Brace Style', scan.braces),
    '',
    section('Comment Forms', scan.comments),
    '',
    section('Doxygen Tags', scan.tags),
    '',
    section('Declaration and Layout Signals', {
      attributes: scan.attributes,
      'callable parameter shapes': scan.callableParameters,
      'single-line controls': scan.singleLineControls,
      'possible missing semicolons': scan.semicolonCandidates,
    }),
    '',
    '## Interpretation Limits',
    '',
    '- A match is not proof that a construct is compiler-valid, attached documentation, or safe to format.',
    '- Semicolon candidates intentionally include false positives and are only a corpus-review queue.',
    '- Generated/proto/native/Workbench classification is heuristic path/content metadata; it must not authorize mutation.',
    '- Use Workbench/compiler validation and parser-backed fixtures before enabling edits.',
    '',
  ].join('\n');
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) return printHelp();
  const { files, scan } = await scanCorpus(args.scripts);
  if (!files.length) throw new Error(`No .c files found under: ${args.scripts}`);
  await fs.mkdir(path.dirname(args.out), { recursive: true });
  await fs.writeFile(args.out, render(args.scripts, files, scan), 'utf8');
  console.log(`Wrote comment formatting corpus report: ${args.out}`);
  console.log(`Scanned ${files.length} .c files.`);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
