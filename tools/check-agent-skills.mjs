#!/usr/bin/env node

import { validateAgentSkills } from "./agent-skills.mjs";

const result = validateAgentSkills({ repositoryRoot: process.cwd() });
if (result.errors.length > 0) {
  console.error(result.errors.join("\n"));
  process.exit(1);
}

console.log(`Agent Skills verified (${result.skillNames.length} skills, ${result.files.length} files).`);
