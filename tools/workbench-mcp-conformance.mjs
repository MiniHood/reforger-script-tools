import { createInterface } from "node:readline";
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { isDeepStrictEqual } from "node:util";
import { spawn } from "node:child_process";
import { arch, platform, release } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const defaultApiReference = join(repositoryRoot, "docs", "mcp-api.md");
const defaultReportPath = join(
  repositoryRoot,
  ".cache",
  "reports",
  "workbench-mcp-contract.json",
);
const defaultServerCandidates = [
  join(repositoryRoot, "server", "target", "debug", "reforger_language_server.exe"),
  join(repositoryRoot, "server", "target", "release", "reforger_language_server.exe"),
  join(repositoryRoot, "dist", "server", "win32-x64", "reforger_language_server.exe"),
];

const workbenchRouterRow =
  /^\| \[\x60(workbench_[^\x60]+)\x60\]\(mcp-api\/tools\/(workbench_[^)]+)\.md\) \|/;

const toolFamilyRules = [
  ["lifecycle", /^(launch|stop|restart)$/],
  ["maintenance", /^(install_bridge|reload|read_logs)$/],
  ["status", /^(status|state|project_context)$/],
  ["validation", /^validate_scripts$/],
  ["resource", /^(inspect_resource|list_resources|search_resources|open_resource)$/],
  ["editor", /^(list_editors|open_editor)$/],
  [
    "world-read",
    /^(world_selection_summary|selected_entity_hierarchy|list_entities|search_world_entities|layer_state|find_entities_by_radius|sample_terrain|get_viewport_context|trace|inspect_prefab_context|inspect_prefab_component|inspect_entity|list_components|inspect_component|list_entity_properties|get_shape_points)$/,
  ],
  ["prefab-write", /^(create_(generic_)?|save_)prefab$|^(add|remove|set)_prefab_/],
  [
    "entity-write",
    /^(set|clear)_selection$|^(create|rename|delete|move|rotate|transform|reparent|duplicate)_entity$|^(add|set|remove)_(component|entity)/,
  ],
  ["history", /^(undo|redo)$/],
  ["shape-write", /^(edit_shape_points|set_polyline_regular_polygon|convert_shape_points|transform_shape_points|resample_polyline)$/],
  ["play-session", /^(start|stop)_play_session$/],
  ["save", /^save$/],
  ["window", /^(list_windows|capture_window)$/],
];

const corpusWorkflowTools = {
  "owned-process": ["workbench_launch", "workbench_status", "workbench_list_windows", "workbench_capture_window"],
  readiness: ["workbench_install_bridge", "workbench_project_context", "workbench_validate_scripts", "workbench_list_editors", "workbench_open_editor"],
  "world-read": ["workbench_search_resources", "workbench_list_resources", "workbench_inspect_resource", "workbench_open_resource", "workbench_state", "workbench_world_selection_summary", "workbench_list_entities", "workbench_search_world_entities", "workbench_layer_state", "workbench_find_entities_by_radius", "workbench_sample_terrain", "workbench_get_viewport_context", "workbench_trace"],
  entity: ["workbench_selected_entity_hierarchy", "workbench_inspect_entity", "workbench_list_components", "workbench_inspect_component", "workbench_list_entity_properties", "workbench_create_entity", "workbench_add_component", "workbench_set_component_properties", "workbench_set_entity_properties", "workbench_rename_entity", "workbench_move_entity", "workbench_rotate_entity", "workbench_transform_entity", "workbench_duplicate_entity", "workbench_reparent_entity", "workbench_set_selection", "workbench_clear_selection", "workbench_remove_component", "workbench_delete_entity", "workbench_undo", "workbench_redo"],
  shape: ["workbench_get_shape_points", "workbench_edit_shape_points", "workbench_set_polyline_regular_polygon", "workbench_convert_shape_points", "workbench_transform_shape_points", "workbench_resample_polyline"],
  "prefab-resource": ["workbench_create_prefab", "workbench_create_generic_prefab", "workbench_inspect_prefab_context", "workbench_inspect_prefab_component", "workbench_add_prefab_resource_component", "workbench_set_prefab_resource_property", "workbench_remove_prefab_resource_component", "workbench_save_prefab"],
  "prefab-editor": ["workbench_set_prefab_property", "workbench_set_prefab_component_property"],
  "save-play-reload": ["workbench_save", "workbench_start_play_session", "workbench_stop_play_session", "workbench_reload", "workbench_read_logs"],
  lifecycle: ["workbench_restart", "workbench_stop"],
};

const corpusWorkflowDependencies = {
  "owned-process": ["catalogue"],
  readiness: ["connectedWorkbench"],
  "world-read": ["activeWorld"],
  entity: ["entity"],
  shape: ["shape"],
  "prefab-resource": ["prefabResource"],
  "prefab-editor": ["prefabEditEntity"],
  "save-play-reload": ["savedWorld"],
  lifecycle: ["replacementProcess"],
};

const corpusFactProducers = {
  catalogue: "<tools/list>",
  ownedProcess: "workbench_launch",
  connectedWorkbench: "workbench_status",
  managedBridge: "workbench_install_bridge",
  projectContext: "workbench_project_context",
  worldEditor: "workbench_open_editor",
  activeWorld: "workbench_open_resource",
  canonicalResource: "workbench_search_resources",
  entity: "workbench_create_entity",
  component: "workbench_add_component",
  shape: "workbench_create_entity",
  prefabResource: "workbench_create_generic_prefab",
  prefabEditEntity: "workbench_inspect_prefab_context",
  savedWorld: "workbench_save",
  playSession: "workbench_start_play_session",
  reloadedRuntime: "workbench_reload",
  replacementProcess: "workbench_restart",
  window: "workbench_list_windows",
};

const corpusWorkflowByTool = Object.fromEntries(
  Object.entries(corpusWorkflowTools).flatMap(([workflow, tools]) =>
    tools.map((tool) => [tool, workflow]),
  ),
);

const corpusWorkflowOrder = [
  "owned-process",
  "readiness",
  "world-read",
  "entity",
  "shape",
  "prefab-resource",
  "prefab-editor",
  "save-play-reload",
  "lifecycle",
];

export function groupWorkbenchScenarioSteps(steps, plan) {
  const workflowByTool = new Map(plan.map((entry) => [entry.tool, entry.workflow]));
  const grouped = new Map();
  for (const step of steps ?? []) {
    const workflow = step.workflow ?? workflowByTool.get(step.tool) ?? "unplanned";
    const workflowSteps = grouped.get(workflow) ?? [];
    workflowSteps.push(step);
    grouped.set(workflow, workflowSteps);
  }
  return [
    ...corpusWorkflowOrder,
    ...[...grouped.keys()].filter((workflow) => !corpusWorkflowOrder.includes(workflow)),
  ]
    .filter((workflow) => grouped.has(workflow))
    .map((workflow) => ({ name: workflow, steps: grouped.get(workflow) }));
}

const corpusToolDependencies = Object.fromEntries([
  [["workbench_status", "workbench_list_windows"], ["ownedProcess"]],
  [["workbench_capture_window"], ["window"]],
  [["workbench_install_bridge"], ["connectedWorkbench"]],
  [["workbench_project_context"], ["managedBridge"]],
  [["workbench_validate_scripts"], ["managedBridge", "projectContext"]],
  [["workbench_list_editors"], ["managedBridge"]],
  [["workbench_open_editor"], ["managedBridge"]],
  [["workbench_search_resources", "workbench_list_resources"], ["managedBridge", "projectContext"]],
  [["workbench_inspect_resource"], ["canonicalResource"]],
  [["workbench_open_resource"], ["canonicalResource", "worldEditor"]],
  [["workbench_state"], ["managedBridge"]],
  [["workbench_world_selection_summary", "workbench_list_entities", "workbench_search_world_entities", "workbench_layer_state", "workbench_find_entities_by_radius", "workbench_sample_terrain", "workbench_get_viewport_context", "workbench_trace"], ["activeWorld"]],
  [["workbench_selected_entity_hierarchy", "workbench_inspect_entity", "workbench_list_components", "workbench_inspect_component", "workbench_list_entity_properties"], ["entity"]],
  [["workbench_create_entity"], ["activeWorld"]],
  [["workbench_add_component", "workbench_set_component_properties", "workbench_set_entity_properties", "workbench_rename_entity", "workbench_move_entity", "workbench_rotate_entity", "workbench_transform_entity", "workbench_reparent_entity", "workbench_duplicate_entity", "workbench_set_selection", "workbench_clear_selection", "workbench_remove_component", "workbench_delete_entity", "workbench_undo", "workbench_redo"], ["entity"]],
  [["workbench_get_shape_points", "workbench_edit_shape_points", "workbench_set_polyline_regular_polygon", "workbench_convert_shape_points", "workbench_transform_shape_points", "workbench_resample_polyline"], ["shape"]],
  [["workbench_create_prefab"], ["entity"]],
  [["workbench_create_generic_prefab"], ["activeWorld"]],
  [["workbench_add_prefab_resource_component", "workbench_remove_prefab_resource_component", "workbench_set_prefab_resource_property"], ["prefabResource"]],
  [["workbench_start_play_session"], ["savedWorld"]],
  [["workbench_stop_play_session"], ["playSession"]],
  [["workbench_reload"], ["savedWorld", "managedBridge"]],
  [["workbench_read_logs"], ["reloadedRuntime"]],
  [["workbench_save"], ["activeWorld"]],
  [["workbench_restart"], ["ownedProcess", "savedWorld"]],
  [["workbench_stop"], ["replacementProcess"]],
].flatMap(([tools, dependencies]) => tools.map((tool) => [tool, dependencies])));

const corpusCaseKinds = {
  workbench_install_bridge: [{ id: "success", kind: "success" }, { id: "consent-guard", kind: "guard" }],
  workbench_create_prefab: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_create_generic_prefab: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_save_prefab: [{ id: "resource-success", kind: "success", dependencies: ["prefabResource"] }, { id: "editor-success", kind: "success", dependencies: ["prefabEditEntity"] }, { id: "confirmation-safety", kind: "guard" }],
  workbench_inspect_prefab_context: [{ id: "resource-success", kind: "success", dependencies: ["prefabResource"] }, { id: "target-guard", kind: "guard" }],
  workbench_inspect_prefab_component: [{ id: "resource-success", kind: "success", dependencies: ["prefabResource"] }, { id: "target-guard", kind: "guard" }],
  workbench_add_prefab_resource_component: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_remove_prefab_resource_component: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_set_prefab_resource_property: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_set_prefab_property: [{ id: "success", kind: "success", dependencies: ["prefabEditEntity"] }, { id: "outside-edit-guard", kind: "guard", dependencies: [] }],
  workbench_set_prefab_component_property: [{ id: "success", kind: "success", dependencies: ["prefabEditEntity"] }, { id: "outside-edit-guard", kind: "guard", dependencies: [] }],
  workbench_remove_component: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_delete_entity: [{ id: "success", kind: "success" }, { id: "confirmation-safety", kind: "guard" }],
  workbench_restart: [{ id: "success", kind: "success" }],
  workbench_stop: [{ id: "success", kind: "success" }],
};

const corpusReadbackTools = {
  workbench_create_entity: ["workbench_inspect_entity"],
  workbench_add_component: ["workbench_list_components", "workbench_inspect_component"],
  workbench_set_component_properties: ["workbench_inspect_component"],
  workbench_set_entity_properties: ["workbench_list_entity_properties"],
  workbench_rename_entity: ["workbench_inspect_entity", "workbench_search_world_entities"],
  workbench_move_entity: ["workbench_inspect_entity"],
  workbench_rotate_entity: ["workbench_inspect_entity", "workbench_list_entity_properties"],
  workbench_transform_entity: ["workbench_inspect_entity"],
  workbench_undo: ["workbench_inspect_entity"],
  workbench_redo: ["workbench_inspect_entity"],
  workbench_reparent_entity: ["workbench_selected_entity_hierarchy", "workbench_inspect_entity"],
  workbench_duplicate_entity: ["workbench_inspect_entity"],
  workbench_set_selection: ["workbench_world_selection_summary"],
  workbench_clear_selection: ["workbench_world_selection_summary"],
  workbench_remove_component: ["workbench_list_components"],
  workbench_delete_entity: ["workbench_inspect_entity"],
  workbench_edit_shape_points: ["workbench_get_shape_points"],
  workbench_set_polyline_regular_polygon: ["workbench_get_shape_points"],
  workbench_convert_shape_points: ["workbench_get_shape_points"],
  workbench_transform_shape_points: ["workbench_get_shape_points"],
  workbench_resample_polyline: ["workbench_get_shape_points"],
  workbench_create_prefab: ["workbench_inspect_resource"],
  workbench_create_generic_prefab: ["workbench_inspect_resource"],
  workbench_add_prefab_resource_component: ["workbench_inspect_prefab_context"],
  workbench_remove_prefab_resource_component: ["workbench_inspect_prefab_context"],
  workbench_set_prefab_resource_property: ["workbench_inspect_prefab_context"],
  workbench_set_prefab_property: ["workbench_inspect_prefab_context"],
  workbench_set_prefab_component_property: ["workbench_inspect_prefab_component"],
  workbench_save_prefab: ["workbench_inspect_prefab_context"],
  workbench_start_play_session: ["workbench_state"],
  workbench_stop_play_session: ["workbench_state"],
  workbench_reload: ["workbench_read_logs"],
  workbench_save: ["workbench_state"],
};

const corpusCleanupTools = {
  workbench_create_entity: ["workbench_delete_entity"],
  workbench_add_component: ["workbench_remove_component"],
  workbench_set_component_properties: ["workbench_delete_entity"],
  workbench_set_entity_properties: ["workbench_delete_entity"],
  workbench_rename_entity: ["workbench_delete_entity"],
  workbench_move_entity: ["workbench_delete_entity"],
  workbench_rotate_entity: ["workbench_delete_entity"],
  workbench_transform_entity: ["workbench_delete_entity"],
  workbench_undo: ["workbench_redo"],
  workbench_redo: ["workbench_undo"],
  workbench_reparent_entity: ["workbench_delete_entity"],
  workbench_duplicate_entity: ["workbench_delete_entity"],
  workbench_edit_shape_points: ["workbench_edit_shape_points"],
  workbench_set_polyline_regular_polygon: ["workbench_delete_entity"],
  workbench_convert_shape_points: [],
  workbench_transform_shape_points: ["workbench_edit_shape_points"],
  workbench_resample_polyline: ["workbench_delete_entity"],
};

export function buildWorkbenchEndpointPlan(
  expectedNames = extractWorkbenchToolNames(readFileSync(defaultApiReference, "utf8")),
) {
  return expectedNames.map((tool) => {
    const workflow = corpusWorkflowByTool[tool] ?? "uncategorized";
    return {
      tool,
      workflow,
      dependencies: [...(corpusToolDependencies[tool] ?? corpusWorkflowDependencies[workflow] ?? [])],
      requiredFacts: [...(corpusToolDependencies[tool] ?? corpusWorkflowDependencies[workflow] ?? [])],
      cases: (corpusCaseKinds[tool] ?? [{ id: "success", kind: "success" }]).map(
        (acceptanceCase) => ({
          ...acceptanceCase,
          readbackTools: acceptanceCase.kind === "success"
            ? [...(corpusReadbackTools[tool] ?? [])]
            : [],
          cleanupTools: acceptanceCase.kind === "success"
            ? [...(corpusCleanupTools[tool] ?? [])]
            : [],
        }),
      ),
    };
  });
}

export function validateWorkbenchEndpointPlan(expectedNames, plan, options = {}) {
  const actualNames = plan.map((entry) => entry?.tool);
  const expected = new Set(expectedNames);
  const actual = new Set(actualNames);
  const missing = expectedNames.filter((name) => !actual.has(name));
  const unexpected = actualNames.filter(
    (name) => typeof name !== "string" || !expected.has(name),
  );
  const duplicates = [...new Set(actualNames.filter(
    (name, index, names) => names.indexOf(name) !== index,
  ))];
  const invalid = [];
  const plannedTools = new Set(actualNames);
  for (const entry of plan) {
    const reasons = [];
    if (!entry || typeof entry.tool !== "string") {
      reasons.push("missing tool");
    }
    if (typeof entry?.workflow !== "string" || entry.workflow === "uncategorized") {
      reasons.push("missing workflow");
    }
    if (!Array.isArray(entry?.dependencies)) {
      reasons.push("missing dependencies");
    } else if (options.checkDependencyProducers === true) {
      for (const dependency of [
        ...entry.dependencies,
        ...entry.cases.flatMap((acceptanceCase) => acceptanceCase?.dependencies ?? []),
      ]) {
        const producer = corpusFactProducers[dependency];
        if (!producer) {
          reasons.push("unknown dependency " + dependency);
        } else if (producer !== "<tools/list>" && !plannedTools.has(producer)) {
          reasons.push(
            "dependency " + dependency + " has no planned producer " + producer,
          );
        }
      }
    }
    if (options.requireEvidenceSchema === true && !Array.isArray(entry?.requiredFacts)) {
      reasons.push("missing required facts");
    }
    if (options.requireEvidenceSchema === true) {
      for (const acceptanceCase of entry?.cases ?? []) {
        for (const field of ["readbackTools", "cleanupTools"]) {
          if (!Array.isArray(acceptanceCase?.[field])) {
            reasons.push("acceptance case missing " + field);
          }
        }
      }
    }
    if (!Array.isArray(entry?.cases) || entry.cases.length === 0) {
      reasons.push("missing acceptance cases");
    } else {
      const caseIds = entry.cases.map((acceptanceCase) => acceptanceCase?.id);
      if (caseIds.some((id) => typeof id !== "string" || id.length === 0)) {
        reasons.push("acceptance case missing id");
      }
      if (new Set(caseIds).size !== caseIds.length) {
        reasons.push("duplicate acceptance case");
      }
      if (!entry.cases.some((acceptanceCase) => acceptanceCase?.kind === "success")) {
        reasons.push("missing successful acceptance case");
      }
    }
    if (reasons.length > 0) {
      invalid.push({ name: entry?.tool ?? null, reasons });
    }
  }
  return {
    ok: missing.length === 0 && unexpected.length === 0 && duplicates.length === 0 && invalid.length === 0,
    missing,
    unexpected: [...new Set(unexpected)],
    duplicates,
    invalid,
  };
}

function inferScenarioCase(step) {
  if (step.case) return step.case;
  const name = String(step.name ?? "").toLowerCase();
  if (name.includes("preview") || name.includes("replay")) return "confirmation-safety";
  if (name.includes("without-target")) return "target-guard";
  if (name.includes("unknown-process")) {
    return step.tool === "workbench_stop" || step.tool === "workbench_restart"
      ? "ownership-guard"
      : "outside-edit-guard";
  }
  if (name.includes("outside-edit")) {
    return "outside-edit-guard";
  }
  if (step.tool === "workbench_save_prefab") {
    return step.arguments?.resourceName ? "resource-success" : "editor-success";
  }
  if (
    step.tool === "workbench_inspect_prefab_context" ||
    step.tool === "workbench_inspect_prefab_component"
  ) {
    return step.arguments?.resourceName ? "resource-success" : "editor-success";
  }
  if (name.includes("edit-context") || name.includes("prefab-edit")) return "editor-success";
  return "success";
}

function inferScenarioRole(step) {
  if (step.role) return step.role;
  const name = String(step.name ?? "").toLowerCase();
  if (name.includes("preview")) return "test";
  if (name.includes("read-after") || name.startsWith("verify-")) {
    return "readback";
  }
  if (step.tool === "workbench_delete_entity" && !name.includes("preview") && !name.startsWith("inspect-")) {
    return "test";
  }
  if (step.tool === "workbench_remove_component" && !name.includes("preview")) {
    return "test";
  }
  if (name.includes("delete-") || name.includes("restore-") || name === "clear-selection") {
    return "teardown";
  }
  return "test";
}

function inferScenarioRoles(step) {
  if (Array.isArray(step.roles) && step.roles.length > 0) {
    return [...step.roles];
  }
  const role = inferScenarioRole(step);
  const roles = [role];
  const serves = inferScenarioServes(step);
  if (serves.length > 0 && !roles.includes("readback")) {
    roles.push("readback");
  }
  if (
    (step.tool === "workbench_delete_entity" || step.tool === "workbench_remove_component") &&
    !String(step.name ?? "").toLowerCase().includes("preview") &&
    !roles.includes("teardown")
  ) {
    roles.push("teardown");
  }
  return roles;
}

function observationHasRole(observation, role) {
  return observation.role === role || observation.roles?.includes(role);
}

function inferScenarioFacts(step) {
  const captured = Object.keys(step.capture ?? {});
  const facts = [];
  if (captured.some((name) => /entityId/i.test(name))) facts.push("entity");
  if (captured.some((name) => /shapeEntityId/i.test(name))) facts.push("shape");
  if (captured.some((name) => /componentId/i.test(name))) facts.push("component");
  if (captured.some((name) => /polylineEntityId|shapePoints/i.test(name))) facts.push("shape");
  if (captured.some((name) => /windowId/i.test(name))) facts.push("window");
  if (captured.some((name) => /prefabResourceName/i.test(name))) facts.push("prefabResource");
  if (captured.some((name) => /ResourceName$/i.test(name))) facts.push("canonicalResource");
  const toolFacts = {
    workbench_install_bridge: "managedBridge",
    workbench_project_context: "projectContext",
    workbench_open_editor: "worldEditor",
    workbench_open_resource: "activeWorld",
    workbench_create_entity: "entity",
    workbench_create_prefab: "prefabResource",
    workbench_add_component: "component",
    workbench_get_shape_points: "shape",
    workbench_create_generic_prefab: "prefabResource",
    workbench_save: "savedWorld",
    workbench_start_play_session: "playSession",
    workbench_reload: "reloadedRuntime",
    workbench_restart: "replacementProcess",
  };
  if (toolFacts[step.tool]) facts.push(toolFacts[step.tool]);
  if (step.tool === "workbench_inspect_prefab_context" && step.arguments?.resourceName) {
    facts.push("prefabResource");
  }
  return facts;
}

function inferScenarioServes(step) {
  if (Array.isArray(step.serves)) return [...step.serves];
  const name = String(step.name ?? "").toLowerCase();
  const direct = [
    ["inspect-created-entity", ["workbench_create_entity"]],
    ["inspect-created-prefab-resource", ["workbench_create_prefab"]],
    ["inspect-generic-resource", ["workbench_create_generic_prefab"]],
    ["list-created-components", ["workbench_create_entity"]],
    ["inspect-created-components", ["workbench_add_component"]],
    ["inspect-component", ["workbench_add_component"]],
    ["read-after-component-property-set", ["workbench_set_component_properties"]],
    ["read-after-entity-property-set", ["workbench_set_entity_properties"]],
    ["read-after-rename", ["workbench_rename_entity"]],
    ["search-after-rename", ["workbench_rename_entity"]],
    ["read-after-move", ["workbench_move_entity"]],
     ["read-after-rotate", ["workbench_rotate_entity"]],
     ["read-after-undo", ["workbench_undo"]],
     ["read-after-redo", ["workbench_redo"]],
    ["read-after-duplicate", ["workbench_duplicate_entity"]],
    ["selected-hierarchy", ["workbench_reparent_entity", "workbench_set_selection"]],
    ["selection-summary-after-clear", ["workbench_clear_selection"]],
    ["read-after-component-remove", ["workbench_remove_component"]],
    ["read-after-polygon", ["workbench_set_polyline_regular_polygon"]],
    ["read-after-resample", ["workbench_resample_polyline"]],
    ["restore-shape-after-transform", ["workbench_transform_shape_points"]],
    ["restore-shape-after-resample", ["workbench_resample_polyline"]],
    ["read-after-prefab-save", ["workbench_save_prefab"]],
    ["read-after-prefab-resource-property-set", ["workbench_set_prefab_resource_property"]],
    ["read-after-prefab-resource-component-add", ["workbench_add_prefab_resource_component"]],
    ["read-after-prefab-resource-component-remove", ["workbench_remove_prefab_resource_component"]],
    ["inspect-generic-prefab-component-context", ["workbench_add_prefab_resource_component"]],
    ["verify-world-after-save", ["workbench_save"]],
    ["inspect-deleted-entity", ["workbench_delete_entity"]],
  ];
  const matched = direct.find(([prefix]) => name.startsWith(prefix));
  if (matched) return [...matched[1]];
  if (name.includes("shape") && step.tool === "workbench_get_shape_points") {
    return [
      "workbench_edit_shape_points",
      "workbench_convert_shape_points",
      "workbench_transform_shape_points",
    ];
  }
  if (step.tool === "workbench_state" && name.includes("play")) {
    return ["workbench_stop_play_session"];
  }
  return [];
}

export function buildWorkbenchCorpusReport(plan, scenarios = [], options = {}) {
  const observationsByTool = new Map();
  for (const scenario of scenarios) {
    for (const step of scenario.steps ?? []) {
      const observations = observationsByTool.get(step.tool) ?? [];
      observations.push({
        ...step,
        case: step.case ?? null,
        role: step.role ?? null,
      });
      observationsByTool.set(step.tool, observations);
    }
  }
  const factState = new Map();
  const attemptedFacts = new Set();
  for (const [tool, observations] of observationsByTool) {
    const producedFacts = Object.entries(corpusFactProducers)
      .filter(([, producer]) => producer === tool)
      .map(([fact]) => fact);
    for (const fact of producedFacts) {
      attemptedFacts.add(fact);
      const explicitlyCaptured = observations.some((observation) =>
        observation.facts?.includes(fact),
      );
      if (!explicitlyCaptured) {
        continue;
      }
      if (observations.some((observation) =>
        observation.outcome === "success",
      )) {
        factState.set(fact, "available");
      } else if (observations.some((observation) =>
        observation.outcome === "failure" || observation.outcome === "expected-unavailable",
      )) {
        factState.set(fact, "blocked");
      }
    }
  }
  for (const fact of options.blockedFacts ?? []) {
    factState.set(fact, "blocked");
  }
  const endpoints = plan.map((entry) => {
    const observations = observationsByTool.get(entry.tool) ?? [];
    const blockedDependencies = entry.dependencies.filter((dependency) =>
      factState.get(dependency) === "blocked" ||
      (factState.get(dependency) !== "available" && attemptedFacts.has(dependency)),
    );
    const cases = entry.cases.map((acceptanceCase) => {
      const matching = observations.filter((observation) => observation.case === acceptanceCase.id);
      const caseDependencies = acceptanceCase.dependencies ?? entry.dependencies;
      const caseBlockedDependencies = caseDependencies.filter((dependency) =>
        factState.get(dependency) === "blocked" ||
        (factState.get(dependency) !== "available" && attemptedFacts.has(dependency)),
      );
      const failure = matching.find((observation) => observation.outcome === "failure");
      const unavailable = matching.find((observation) => observation.outcome === "expected-unavailable");
      const allowedRoles = acceptanceCase.roles ??
        (acceptanceCase.kind === "guard" ? ["test"] : ["test", "setup"]);
      const passing = matching.some((observation) =>
        allowedRoles.some((role) => observationHasRole(observation, role)) &&
        (observation.outcome === "success" ||
          (acceptanceCase.kind === "guard" &&
            ["expected-error", "expected-unavailable"].includes(observation.outcome))),
      );
      const readbackEvidence = (acceptanceCase.readbackTools ?? []).flatMap((tool) =>
        observationsByTool.get(tool) ?? [],
      ).filter((observation) =>
        observationHasRole(observation, "readback") &&
        observation.outcome === "success" &&
        observation.serves?.includes(entry.tool),
      );
      const missingReadbackTools = (acceptanceCase.readbackTools ?? []).filter((tool) =>
        !readbackEvidence.some((observation) => observation.tool === tool),
      );
      const cleanupEvidence = (acceptanceCase.cleanupTools ?? []).flatMap((tool) =>
        observationsByTool.get(tool) ?? [],
      ).filter((observation) =>
        observationHasRole(observation, "teardown") &&
        observation.outcome === "success" &&
        (matching.length === 0 ||
          matching.some((mutation) =>
            mutation.target === null ||
            observation.target === mutation.target,
          )),
      );
      const missingCleanupTools = (acceptanceCase.cleanupTools ?? []).filter((tool) =>
        !cleanupEvidence.some((observation) => observation.tool === tool),
      );
      const evidenceFailure = passing &&
        (missingReadbackTools.length > 0 || missingCleanupTools.length > 0);
      return {
        ...acceptanceCase,
        status: failure || evidenceFailure ? "failed" :
          caseBlockedDependencies.length > 0
          ? "blocked"
          : passing
            ? "passed"
            : unavailable && acceptanceCase.kind !== "guard"
              ? "blocked"
              : "not-run",
        dependencies: caseDependencies,
        blockers: [
          ...caseBlockedDependencies,
          ...missingReadbackTools.map((tool) => "readback:" + tool),
          ...missingCleanupTools.map((tool) => "cleanup:" + tool),
        ],
        readbackEvidence,
        cleanupEvidence,
        observations: matching,
      };
    });
    const status = cases.some((acceptanceCase) => acceptanceCase.status === "failed")
      ? "failed"
      : cases.some((acceptanceCase) => acceptanceCase.status === "blocked")
        ? "blocked"
        : blockedDependencies.length > 0
          ? "blocked"
        : cases.some((acceptanceCase) => acceptanceCase.status === "not-run")
          ? "not-run"
          : "passed";
    return {
      tool: entry.tool,
      workflow: entry.workflow,
      dependencies: entry.dependencies,
      requiredFacts: entry.requiredFacts ?? entry.dependencies,
      status,
      cases,
      invocations: observations,
      blockers: [
        ...blockedDependencies,
        ...cases.flatMap((acceptanceCase) => acceptanceCase.blockers ?? []),
        ...observations.flatMap((observation) => observation.blockedBy ?? []),
      ],
    };
  });
  const statuses = ["passed", "failed", "blocked", "not-run"];
  const counts = Object.fromEntries(statuses.map((status) => [
    status,
    endpoints.filter((endpoint) => endpoint.status === status).length,
  ]));
  return {
    ok: endpoints.every((endpoint) => endpoint.status === "passed"),
    endpointCount: endpoints.length,
    counts,
    passed: endpoints.filter((endpoint) => endpoint.status === "passed").map((endpoint) => endpoint.tool),
    failed: endpoints.filter((endpoint) => endpoint.status === "failed").map((endpoint) => endpoint.tool),
    blocked: endpoints.filter((endpoint) => endpoint.status === "blocked").map((endpoint) => endpoint.tool),
    notRun: endpoints.filter((endpoint) => endpoint.status === "not-run").map((endpoint) => endpoint.tool),
    "not-run": endpoints.filter((endpoint) => endpoint.status === "not-run").map((endpoint) => endpoint.tool),
    endpoints,
  };
}

export function extractWorkbenchToolNames(apiReference) {
  const names = [];
  for (const line of apiReference.split(/\r?\n/)) {
    const match = workbenchRouterRow.exec(line);
    if (match && match[1] === match[2]) {
      names.push(match[1]);
    }
  }
  return names;
}

export function buildContractReport({ apiReference, listedTools }) {
  const expectedNames = extractWorkbenchToolNames(apiReference);
  const workbenchTools = listedTools.filter(
    (tool) => typeof tool?.name === "string" && tool.name.startsWith("workbench_"),
  );
  const actualNames = workbenchTools.map((tool) => tool.name);
  const expected = new Set(expectedNames);
  const actual = new Set(actualNames);
  const missing = expectedNames.filter((name) => !actual.has(name));
  const unexpected = actualNames
    .filter((name) => !expected.has(name))
    .filter((name, index, names) => names.indexOf(name) === index);
  const duplicates = actualNames.filter(
    (name, index, names) => names.indexOf(name) !== index,
  );
  const invalid = workbenchTools
    .map((tool) => {
      const reasons = [];
      if (typeof tool.description !== "string" || tool.description.length === 0) {
        reasons.push("missing description");
      }
      if (!isObject(tool.annotations)) {
        reasons.push("missing annotations");
      }
      if (!isObject(tool.inputSchema)) {
        reasons.push("missing inputSchema");
      }
      if (!isObject(tool.outputSchema)) {
        reasons.push("missing outputSchema");
      }
      return reasons.length === 0 ? undefined : { name: tool.name, reasons };
    })
    .filter(Boolean);

  const coverage = expectedNames.map((name) => ({
    tool: name,
    family: classifyTool(name),
    contractEvidence: actual.has(name) && !invalid.some((item) => item.name === name)
      ? "tools/list"
      : "missing",
    liveEvidence: "not-run",
  }));
  const uncategorized = coverage
    .filter((entry) => entry.family === "uncategorized")
    .map((entry) => entry.tool);

  return {
    ok:
      missing.length === 0 &&
      unexpected.length === 0 &&
      duplicates.length === 0 &&
      invalid.length === 0 &&
      uncategorized.length === 0,
    expectedNames,
    actualNames,
    missing,
    unexpected,
    duplicates: [...new Set(duplicates)],
    invalid,
    expectedCount: expectedNames.length,
    actualCount: actualNames.length,
    coverage,
    uncategorized,
    apiReferenceFingerprint: fingerprint(apiReference),
    liveCatalogueFingerprint: fingerprint(listedTools),
  };
}

export function classifyTool(name) {
  const suffix = name.startsWith("workbench_")
    ? name.slice("workbench_".length)
    : name;
  const match = toolFamilyRules.find(([, rule]) => rule.test(suffix));
  return match?.[0] ?? "uncategorized";
}

export function percentile(samples, percentileValue) {
  if (samples.length === 0) {
    return null;
  }
  const sorted = [...samples].sort((left, right) => left - right);
  const rank = Math.max(0, Math.ceil(percentileValue * sorted.length) - 1);
  return sorted[rank];
}

export function summarizeSamples(samples) {
  const values = samples
    .map((sample) => Number(sample))
    .filter((sample) => Number.isFinite(sample) && sample >= 0);
  if (values.length === 0) {
    return {
      count: 0,
      minimumMs: null,
      maximumMs: null,
      p50Ms: null,
      p95Ms: null,
      p99Ms: null,
    };
  }
  return {
    count: values.length,
    minimumMs: Math.min(...values),
    maximumMs: Math.max(...values),
    p50Ms: percentile(values, 0.5),
    p95Ms: percentile(values, 0.95),
    p99Ms: percentile(values, 0.99),
  };
}

export function summarizePerformance(scenarios) {
  const byTool = new Map();
  for (const scenario of scenarios) {
    for (const step of scenario.steps ?? []) {
      const entry = byTool.get(step.tool) ?? { samples: [], failureCount: 0 };
      entry.samples.push(step.durationMs);
      entry.requestBytes ??= [];
      entry.responseBytes ??= [];
      if (step.requestBytes !== null && step.requestBytes !== undefined) {
        entry.requestBytes.push(step.requestBytes);
      }
      if (step.responseBytes !== null && step.responseBytes !== undefined) {
        entry.responseBytes.push(step.responseBytes);
      }
      if (step.outcome === "failure") {
        entry.failureCount += 1;
      }
      byTool.set(step.tool, entry);
    }
  }
  return [...byTool.entries()].sort(([left], [right]) => left.localeCompare(right)).map(
    ([tool, entry]) => ({
      tool,
      ...summarizeSamples(entry.samples),
      requestBytes: summarizeRange(entry.requestBytes),
      responseBytes: summarizeRange(entry.responseBytes),
      successCount: entry.samples.length - entry.failureCount,
      failureCount: entry.failureCount,
    }),
  );
}

function summarizeRange(values = []) {
  const numeric = values.filter((value) => Number.isFinite(value) && value >= 0);
  return numeric.length === 0
    ? { minimum: null, maximum: null }
    : { minimum: Math.min(...numeric), maximum: Math.max(...numeric) };
}

function isLiveEvidence(step) {
  return step.synthetic !== true && ["success", "expected-error", "expected-unavailable"].includes(
    step.outcome,
  );
}

function validateStructuredError(response, expectedError) {
  if (!expectedError) {
    return [];
  }
  const actual = response?.result?.structuredContent;
  if (!isObject(actual) || typeof actual.code !== "string") {
    return ["expected a structured Workbench error with a stable code"];
  }
  const expectedCodes = expectedError.codes ??
    (expectedError.code ? [expectedError.code] : []);
  const expectedPhases = expectedError.phases ??
    (expectedError.phase ? [expectedError.phase] : []);
  const reasons = [];
  if (expectedCodes.length > 0 && !expectedCodes.includes(actual.code)) {
    reasons.push(
      "expected error code " +
        JSON.stringify(expectedCodes) +
        " but received " +
        JSON.stringify(actual.code),
    );
  }
  if (
    expectedPhases.length > 0 &&
    !expectedPhases.includes(actual.phase)
  ) {
    reasons.push(
      "expected error phase " +
        JSON.stringify(expectedPhases) +
        " but received " +
        JSON.stringify(actual.phase),
    );
  }
  if (
    expectedError.retryable !== undefined &&
    actual.retryable !== expectedError.retryable
  ) {
    reasons.push(
      "expected error retryable=" +
        expectedError.retryable +
        " but received " +
        actual.retryable,
    );
  }
  return reasons;
}

export class McpStdioClient {
  constructor({ serverPath, args = [], env = {}, requestTimeoutMs = 120000 }) {
    this.child = spawn(serverPath, args, {
      cwd: repositoryRoot,
      env: { ...process.env, ...env },
      stdio: ["pipe", "pipe", "pipe"],
      windowsHide: true,
    });
    this.lines = createInterface({ input: this.child.stdout });
    this.messages = this.lines[Symbol.asyncIterator]();
    this.stderr = "";
    this.child.stderr.setEncoding("utf8");
    this.child.stderr.on("data", (chunk) => {
      this.stderr += chunk;
    });
    this.nextId = 1;
    this.requestTimeoutMs = requestTimeoutMs;
  }

  async initialize() {
    const result = await this.request("initialize", {
      protocolVersion: "2025-11-25",
      capabilities: {},
      clientInfo: {
        name: "reforger-script-tools-workbench-conformance",
        version: "1.0.0",
      },
    });
    this.send({
      jsonrpc: "2.0",
      method: "notifications/initialized",
    });
    return result;
  }

  async listTools() {
    return this.request("tools/list", {});
  }

  async callTool(name, argumentsValue = {}) {
    return (await this.callToolTimed(name, argumentsValue)).response;
  }

  async callToolTimed(name, argumentsValue = {}) {
    return this.requestTimed("tools/call", {
      name,
      arguments: argumentsValue,
    });
  }

  async request(method, params) {
    return (await this.requestTimed(method, params)).response;
  }

  async requestTimed(method, params) {
    const id = this.nextId++;
    const requestText =
      JSON.stringify({
        jsonrpc: "2.0",
        id,
        method,
        params,
      }) + "\n";
    const started = performanceNow();
    this.send({
      jsonrpc: "2.0",
      id,
      method,
      params,
    });
    while (true) {
      const next = await new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          reject(
            new Error(
              "MCP request timed out after " +
                this.requestTimeoutMs +
                "ms: " +
                method,
            ),
          );
        }, this.requestTimeoutMs);
        this.messages.next().then(
          (value) => {
            clearTimeout(timer);
            resolve(value);
          },
          (error) => {
            clearTimeout(timer);
            reject(error);
          },
        );
      });
      if (next.done) {
        throw new Error(
          "MCP process ended before responding to " +
            method +
            ": " +
            this.stderr.trim(),
        );
      }
      const message = JSON.parse(next.value);
      if (message.id === id) {
        return {
          response: message,
          timing: {
            durationMs: performanceNow() - started,
            requestBytes: Buffer.byteLength(requestText),
            responseBytes: Buffer.byteLength(next.value),
          },
        };
      }
    }
  }

  send(message) {
    this.child.stdin.write(JSON.stringify(message) + "\n");
  }

  async close() {
    this.lines.close();
    this.child.stdin.end();
    await waitForClose(this.child);
  }
}

export function loadFixtureManifest(manifestPath) {
  const manifestFile = resolve(manifestPath);
  const manifest = JSON.parse(readFileSync(manifestFile, "utf8"));
  if (!isObject(manifest)) {
    throw new Error("Workbench fixture manifest must be an object");
  }
  for (const field of ["name", "revision", "fixtureRoot", "project", "profileRoot"]) {
    if (manifest[field] === undefined) {
      throw new Error("Workbench fixture manifest is missing " + field);
    }
  }
  const manifestRoot = dirname(manifestFile);
  const fixtureRoot = resolve(manifestRoot, manifest.fixtureRoot);
  const projectPath = resolve(fixtureRoot, manifest.project.gproj);
  const profileRoot = resolve(fixtureRoot, manifest.profileRoot);
  const externalProfileRoot = manifest.profileRootOutsideFixture === true
    ? resolve(manifestRoot, manifest.profileRoot)
    : profileRoot;
  const consentGuardProfileRoot = manifest.consentGuardProfileRoot
    ? resolve(fixtureRoot, manifest.consentGuardProfileRoot)
    : undefined;
  const addonsDir = resolve(manifestRoot, manifest.project.addonsDir);
  if (!existsSync(fixtureRoot)) {
    throw new Error("Workbench fixture root does not exist: " + fixtureRoot);
  }
  if (!existsSync(projectPath)) {
    throw new Error("Workbench fixture project does not exist: " + projectPath);
  }
  if (!existsSync(addonsDir)) {
    throw new Error("Workbench fixture add-ons directory does not exist: " + addonsDir);
  }
  if (!isObject(manifest.expected) || typeof manifest.expected.worldResource !== "string") {
    throw new Error("Workbench fixture manifest must define expected.worldResource");
  }
  if (
    !Array.isArray(manifest.expected.loadedAddonIds) ||
    !manifest.expected.loadedAddonIds.includes("ArmaReforger")
  ) {
    throw new Error(
      "Workbench fixture manifest must require the ArmaReforger base-game add-on",
    );
  }
  if (!isWithin(fixtureRoot, projectPath)) {
    throw new Error("Workbench fixture project must be inside fixtureRoot");
  }
  if (!isWithin(fixtureRoot, externalProfileRoot) && manifest.profileRootOutsideFixture !== true) {
    throw new Error("Workbench fixture profileRoot must be inside fixtureRoot");
  }
  if (
    consentGuardProfileRoot &&
    !isWithin(fixtureRoot, consentGuardProfileRoot) &&
    manifest.profileRootOutsideFixture !== true
  ) {
    throw new Error("Workbench consent guard profile must be inside fixtureRoot");
  }
  if (consentGuardProfileRoot && resolve(consentGuardProfileRoot) === resolve(externalProfileRoot)) {
    throw new Error("Workbench consent guard profile must differ from the active profile");
  }
  mkdirSync(externalProfileRoot, { recursive: true });
  if (consentGuardProfileRoot) {
    mkdirSync(consentGuardProfileRoot, { recursive: true });
  }
  return {
    name: String(manifest.name),
    revision: String(manifest.revision),
    fixtureRoot,
    projectPath,
    profileRoot: externalProfileRoot,
    consentGuardProfileRoot,
    useProfile: manifest.useProfile !== false,
    allowExistingProcess: manifest.allowExistingProcess === true,
    addonsDir,
    expected: isObject(manifest.expected) ? manifest.expected : {},
    readiness: {
      timeoutMs: manifest.readiness?.timeoutMs ?? 120000,
      intervalMs: manifest.readiness?.intervalMs ?? 1000,
    },
  };
}

export async function runConsentGuardProbe({ serverPath, profileRoot }) {
  const before = listRelativeEntries(profileRoot);
  if (before.length > 0) {
    throw new Error(
      "Workbench consent guard profile must start empty: " + profileRoot,
    );
  }
  const client = new McpStdioClient({
    serverPath,
    args: ["mcp", "--workbench-profile-directory", profileRoot],
  });
  try {
    await client.initialize();
    const run = await runScenario({
      client,
      name: "consent-guard",
      includeInvocationMetadata: true,
      steps: [{
        name: "install-bridge-without-consent",
        tool: "workbench_install_bridge",
        role: "test",
        case: "consent-guard",
        expect: {
          isError: true,
          error: {
            code: "workbench_installation_consent_required",
            phase: "install",
          },
        },
      }],
    });
    const after = listRelativeEntries(profileRoot);
    const profileEntriesUnchanged = JSON.stringify(before) === JSON.stringify(after);
    if (!profileEntriesUnchanged) {
      run.ok = false;
      run.steps[0].outcome = "failure";
      run.steps[0].reasons = ["consent guard changed profile files"];
    }
    return { ...run, profileRoot, profileEntriesUnchanged };
  } finally {
    try {
      await client.close();
    } catch {
      // Preserve the probe failure when initialization or the call failed.
    }
  }
}

export class WorkbenchMcpSession {
  constructor(manifest) {
    this.manifest = manifest;
    this.processId = null;
    this.ownsProcess = false;
    this.launch = undefined;
  }

  async start(client) {
    const response = await client.callTool("workbench_launch", {
      projectPath: this.manifest.projectPath,
    });
    const launch = response?.result?.structuredContent;
    if (response?.result?.isError === true || !launch) {
      const error = new Error(
        "Workbench MCP launch failed: " +
          JSON.stringify(launch ?? response?.result ?? null),
      );
      error.workbench = launch ?? response?.result?.structuredContent ?? null;
      throw error;
    }
    this.launch = launch;
    this.processId = launch.processId ?? null;
    this.ownsProcess = launch.alreadyRunning === false;
    if (
      !this.processId ||
      !Number.isInteger(this.processId) ||
      this.processId <= 0 ||
      launch.alreadyRunning !== false ||
      launch.netApiConnected !== true
    ) {
      throw new Error(
        "Workbench fixture launch did not prove a connected process: " +
          JSON.stringify(launch),
      );
    }
    if (launch.alreadyRunning === true && !this.manifest.allowExistingProcess) {
      throw new Error(
        "Workbench fixture launch reused an existing process; " +
          "refusing to run without disposable ownership",
      );
    }
    return {
      processId: this.processId,
      ownsProcess: this.ownsProcess,
      launch,
    };
  }

  async stop(client) {
    if (!this.processId || !this.ownsProcess) {
      return {
        outcome: this.processId ? "reused-existing-process" : "no-process",
        processId: this.processId,
      };
    }
    const response = await client.callTool("workbench_stop", {
      processId: this.processId,
    });
    const stopped = response?.result?.structuredContent;
    if (response?.result?.isError === true || !stopped?.exited) {
      throw new Error(
        "Workbench MCP stop did not confirm process exit: " +
          JSON.stringify(stopped ?? response?.result ?? null),
      );
    }
    return {
      outcome: "graceful",
      processId: this.processId,
    };
  }
}

export async function verifyOwnedStopRestartLifecycle({ client, session }) {
  if (!session?.ownsProcess || !session.processId) {
    throw new Error("Owned Workbench lifecycle requires a process started by MCP launch");
  }
  const originalProcessId = session.processId;
  const restartResponse = await client.callTool("workbench_restart", {
    processId: originalProcessId,
  });
  const restarted = restartResponse?.result?.structuredContent;
  if (
    restartResponse?.result?.isError === true ||
    !restarted ||
    restarted.alreadyRunning === true ||
    restarted.netApiConnected !== true ||
    !Number.isInteger(restarted.processId) ||
    restarted.processId <= 0 ||
    restarted.processId === originalProcessId
  ) {
    throw new Error(
      "Workbench MCP restart did not confirm an owned replacement process: " +
        JSON.stringify(restarted ?? restartResponse?.result ?? null),
    );
  }
  session.processId = restarted.processId;

  const stopResponse = await client.callTool("workbench_stop", {
    processId: session.processId,
  });
  const stopped = stopResponse?.result?.structuredContent;
  if (
    stopResponse?.result?.isError === true ||
    !stopped ||
    stopped.exited !== true
  ) {
    throw new Error(
      "Workbench MCP stop did not confirm the restarted owned process exit: " +
        JSON.stringify(stopped ?? stopResponse?.result ?? null),
    );
  }
  const restartedProcessId = session.processId;
  session.processId = null;
  session.ownsProcess = false;
  return {
    outcome: "graceful",
    originalProcessId,
    restartedProcessId,
    restart: restarted,
    stop: stopped,
    invocations: [
      {
        name: "restart-owned-process",
        tool: "workbench_restart",
        role: "test",
        case: "success",
        outcome: "success",
        facts: ["replacementProcess"],
        response: restarted,
      },
      {
        name: "stop-replacement-process",
        tool: "workbench_stop",
        role: "test",
        case: "success",
        outcome: "success",
        facts: [],
        response: stopped,
      },
    ],
  };
}

export async function waitForWorkbenchReady(client, readiness = {}) {
  const timeoutMs = readiness.timeoutMs ?? 120000;
  const intervalMs = readiness.intervalMs ?? 1000;
  const started = performanceNow();
  let attempts = 0;
  let lastError;
  while (performanceNow() - started <= timeoutMs) {
    attempts += 1;
    try {
      const response = await client.callTool("workbench_status", {});
      if (
        response?.result?.isError !== true &&
        response?.result?.structuredContent?.isRunning === true
      ) {
        return {
          ready: true,
          attempts,
          elapsedMs: performanceNow() - started,
        };
      }
      lastError = "Workbench status did not report isRunning=true";
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await delay(intervalMs);
  }
  throw new Error(
    "Workbench NET API readiness timed out after " +
      timeoutMs +
      "ms" +
      (lastError ? ": " + lastError : ""),
  );
}

export async function openFixtureWorld(client, worldResource, readiness = {}) {
  const { editors } = await waitForWorkbenchEditors(client, readiness);
  const worldEditor = editors.find((editor) =>
    /world editor/i.test(String(editor?.displayName ?? "")),
  );
  if (!worldEditor || typeof worldEditor.id !== "string") {
    throw new Error(
      "Fixture Workbench did not expose a World Editor: " +
        JSON.stringify(editors),
    );
  }
  const openEditorResponse = await client.callTool("workbench_open_editor", {
    editorId: worldEditor.id,
  });
  const openedEditor = openEditorResponse?.result?.structuredContent;
  if (
    openEditorResponse?.result?.isError === true ||
    openedEditor?.opened !== true
  ) {
    throw new Error(
      "Fixture Workbench could not open World Editor " +
      JSON.stringify({ editor: worldEditor, response: openedEditor ?? null }),
    );
  }
  await waitForWorkbenchReady(client, readiness);
  const openResourceResponse = await client.callTool("workbench_open_resource", {
    resourcePath: worldResource,
  });
  const openedResource = openResourceResponse?.result?.structuredContent;
  if (
    openResourceResponse?.result?.isError === true ||
    openedResource?.opened !== true
  ) {
    const discoveredResponse = await client.callTool("workbench_search_resources", {
      kinds: ["world"],
      query: "McpConformance",
      limit: 20,
    });
    throw new Error(
      "Fixture Workbench could not open world resource " +
        JSON.stringify({
          worldResource,
          response: openedResource ?? null,
          discovered: discoveredResponse?.result?.structuredContent ?? null,
        }),
    );
  }
  const state = await waitForActiveWorld(client, worldResource, readiness);
  return {
    editor: worldEditor,
    openedEditor,
    openedResource,
    state,
  };
}

export async function waitForWorkbenchEditors(client, readiness = {}) {
  const timeoutMs = readiness.timeoutMs ?? 120000;
  const intervalMs = readiness.intervalMs ?? 1000;
  const started = performanceNow();
  let attempts = 0;
  let lastResponse;
  while (performanceNow() - started <= timeoutMs) {
    attempts += 1;
    try {
      const response = await client.callTool("workbench_list_editors", {});
      const editors = response?.result?.structuredContent?.editors;
      if (response?.result?.isError !== true && Array.isArray(editors)) {
        return {
          editors,
          attempts,
          elapsedMs: performanceNow() - started,
        };
      }
      lastResponse = response?.result?.structuredContent ?? response?.result ?? null;
    } catch (error) {
      lastResponse = error instanceof Error ? error.message : String(error);
    }
    await delay(intervalMs);
  }
  throw new Error(
    "Workbench editor catalogue readiness timed out after " +
      timeoutMs +
      "ms" +
      (lastResponse ? ": " + JSON.stringify(lastResponse) : ""),
  );
}

export async function waitForActiveWorld(client, expectedWorldResource, readiness = {}) {
  const timeoutMs = readiness.timeoutMs ?? 120000;
  const intervalMs = readiness.intervalMs ?? 1000;
  const started = performanceNow();
  let attempts = 0;
  let lastState;
  let lastError;
  while (performanceNow() - started <= timeoutMs) {
    attempts += 1;
    try {
      const response = await client.callTool("workbench_state", {});
      lastState = response?.result?.structuredContent;
      if (
        response?.result?.isError !== true &&
        lastState?.activeWorldPath === expectedWorldResource
      ) {
        return {
          ...lastState,
          attempts,
          elapsedMs: performanceNow() - started,
        };
      }
      lastError = "Workbench state did not report the expected active world";
    } catch (error) {
      lastError = error instanceof Error ? error.message : String(error);
    }
    await delay(intervalMs);
  }
  throw new Error(
    "Fixture active world did not become ready after " +
      timeoutMs +
      "ms: " +
      JSON.stringify({
        expectedWorldResource,
        attempts,
        lastState: lastState ?? null,
        lastError: lastError ?? null,
      }),
  );
}

export async function runScenario({
  client,
  name,
  steps,
  iterations = 1,
  warmup = 0,
  context = {},
  includeInvocationMetadata = false,
  scenarioState,
}) {
  const observations = [];
  let ok = true;
  const scenarioContext = {
    ...context,
    scenario: scenarioState ?? {},
  };
  const totalIterations = Math.max(1, Number(iterations));
  const warmupIterations = Math.max(0, Number(warmup));
  for (let iteration = 0; iteration < warmupIterations + totalIterations; iteration += 1) {
    for (const step of steps) {
      const started = performanceNow();
      let timed;
      let response;
      let materializedArguments;
      const capturedValues = {};
      let reasons = [];
      try {
        materializedArguments = materialize(step.arguments ?? {}, scenarioContext);
        if (step.distinctValueFrom) {
          const baseline = materialize(step.distinctValueFrom, scenarioContext);
          const distinctValue = makeDistinctValue(baseline);
          materializedArguments.value = distinctValue;
          if (step.captureDistinctAs) {
            scenarioContext.scenario[step.captureDistinctAs] = distinctValue;
          }
        }
        timed = await client.callToolTimed(
          step.tool,
          materializedArguments,
        );
        response = timed.response;
        const actualIsError = response?.result?.isError === true;
        const expectedIsError = step.expect?.isError;
        if (
          expectedIsError !== undefined &&
          actualIsError !== expectedIsError
        ) {
          reasons.push(
            "expected isError=" + expectedIsError + " but received " + actualIsError,
          );
        } else if (
          expectedIsError === undefined &&
          actualIsError &&
          step.expect?.allowError !== true
        ) {
          reasons.push("expected a successful result but received isError=true");
        }
        if (actualIsError && step.expect?.allowError === true && !step.expect.error) {
          reasons.push(
            "allowError requires an explicit structured error oracle",
          );
        }
        if (actualIsError) {
          reasons.push(
            ...validateStructuredError(response, step.expect?.error),
          );
        }
        for (const [pointer, expectedValue] of Object.entries(
          step.expect?.pointers ?? {},
        )) {
          const expected = materialize(expectedValue, scenarioContext);
          const actual = readJsonPointer(response, pointer);
          if (!isDeepStrictEqual(actual, expected)) {
            reasons.push(
              "expected " + pointer + " to equal " + JSON.stringify(expected),
            );
          }
        }
        for (const [pointer, expectedValues] of Object.entries(
          step.expect?.contains ?? {},
        )) {
          const actual = readJsonPointer(response, pointer);
          if (
            !Array.isArray(actual) ||
            !expectedValues.every((expectedValue) =>
              actual.includes(materialize(expectedValue, scenarioContext)),
            )
          ) {
            reasons.push(
              "expected " + pointer + " to contain " + JSON.stringify(expectedValues),
            );
          }
        }
        for (const pointer of step.expect?.exists ?? []) {
          if (readJsonPointer(response, pointer) === undefined) {
            reasons.push("expected " + pointer + " to exist");
          }
        }
        if (reasons.length === 0 && isObject(step.capture)) {
          for (const [nameToCapture, pointer] of Object.entries(step.capture)) {
            const captured = readJsonPointer(response, pointer);
            if (captured === undefined) {
              reasons.push(
                "capture " + nameToCapture + " could not read " + pointer,
              );
            } else {
              scenarioContext.scenario[nameToCapture] = captured;
              capturedValues[nameToCapture] = captured;
            }
          }
        }
      } catch (error) {
        reasons = [error instanceof Error ? error.message : String(error)];
      }
      if (iteration < warmupIterations) {
        if (reasons.length > 0) {
          throw new Error(
            "Workbench scenario warmup failed for " +
              step.tool +
              " (" +
              (step.name ?? "unnamed") +
              "): " +
              reasons.join("; "),
          );
        }
        continue;
      }
      const actualIsError = response?.result?.isError === true;
      const observation = {
        name: step.name,
        tool: step.tool,
        outcome:
          reasons.length === 0
            ? step.expect?.completion === false
              ? "expected-unavailable"
              : actualIsError
                ? "expected-error"
                : "success"
            : "failure",
        durationMs: timed?.timing.durationMs ?? performanceNow() - started,
        requestBytes: timed?.timing.requestBytes ?? null,
        responseBytes: timed?.timing.responseBytes ?? null,
      };
      const errorContent = response?.result?.structuredContent;
      if (actualIsError && isObject(errorContent)) {
        observation.error = {
          code: errorContent.code ?? null,
          phase: errorContent.phase ?? null,
          retryable: errorContent.retryable ?? null,
          logReference: errorContent.logReference ?? null,
        };
      }
      if (step.expect?.completion === false) {
        observation.completion = false;
        if (step.expect.completionReason) {
          observation.completionReason = step.expect.completionReason;
        }
      }
      if (totalIterations > 1) {
        observation.iteration = iteration - warmupIterations + 1;
      }
      if (reasons.length > 0) {
        observation.reasons = reasons;
        ok = false;
      }
      if (
        includeInvocationMetadata ||
        step.role !== undefined ||
        step.case !== undefined ||
        step.facts !== undefined ||
        step.blockedBy !== undefined
      ) {
        observation.role = inferScenarioRole(step);
        observation.roles = inferScenarioRoles(step);
        observation.case = inferScenarioCase(step);
        observation.facts = Array.isArray(step.facts)
          ? [...step.facts]
          : inferScenarioFacts(step);
        observation.serves = inferScenarioServes(step);
        observation.arguments = materializedArguments ?? null;
        observation.captures = capturedValues;
        observation.target =
          materializedArguments?.entityId ??
          materializedArguments?.componentId ??
          materializedArguments?.resourceName ??
          materializedArguments?.processId ??
          capturedValues.entityId ??
          capturedValues.componentId ??
          capturedValues.prefabResourceName ??
          Object.entries(capturedValues).find(([name]) => /entityId/i.test(name))?.[1] ??
          null;
        if (Array.isArray(step.blockedBy)) {
          observation.blockedBy = [...step.blockedBy];
        }
      }
      observations.push(observation);
    }
  }
  return {
    name,
    ok,
    iterations: totalIterations,
    warmup: warmupIterations,
    steps: observations,
    performance: summarizePerformance([{ name, steps: observations }]),
  };
}

export async function runWorkbenchWorkflows({
  client,
  steps,
  plan,
  context = {},
  iterations = 1,
  warmup = 0,
  availableFacts = [],
}) {
  const scenarioState = {};
  const workflowDefinitions = groupWorkbenchScenarioSteps(steps, plan);
  const runs = [];
  const facts = new Set(availableFacts);
  for (const workflow of workflowDefinitions) {
    const observations = [];
    let workflowOk = true;
    for (const step of workflow.steps) {
      const planEntry = plan.find((entry) => entry.tool === step.tool);
      const dependencies = step.workflow
        ? (corpusWorkflowDependencies[step.workflow] ?? [])
        : planEntry?.requiredFacts ?? planEntry?.dependencies ?? [];
      const missingFacts = dependencies.filter((fact) =>
        !facts.has(fact) && corpusFactProducers[fact] !== step.tool,
      );
      if (missingFacts.length > 0) {
        observations.push({
          name: step.name,
          tool: step.tool,
          role: inferScenarioRole(step),
          roles: inferScenarioRoles(step),
          case: inferScenarioCase(step),
          facts: [],
          serves: inferScenarioServes(step),
          arguments: null,
          captures: {},
          target: null,
          outcome: "expected-unavailable",
          synthetic: true,
          blockedBy: missingFacts,
          reasons: ["required fact unavailable: " + missingFacts.join(", ")],
        });
        continue;
      }
      const run = await runScenario({
        client,
        name: workflow.name,
        steps: [step],
        iterations,
        warmup,
        context,
        includeInvocationMetadata: true,
        scenarioState,
      });
      observations.push(...run.steps);
      workflowOk &&= run.ok;
      for (const observation of run.steps) {
        if (observation.outcome === "success") {
          for (const fact of observation.facts ?? []) facts.add(fact);
        }
      }
    }
    runs.push({
      name: workflow.name,
      ok: workflowOk,
      iterations,
      warmup,
      steps: observations,
      performance: summarizePerformance([{ name: workflow.name, steps: observations }]),
    });
  }
  return {
    ok: runs.every((run) => run.ok),
    runs,
    scenarioState,
    facts: [...facts],
  };
}

export async function runWorkbenchCorpus({
  serverPath = resolveServerPath(),
  apiReferencePath = defaultApiReference,
  reportPath = defaultReportPath,
  indexCachePath,
  scenarioPath,
  fixturePath,
  requireLiveCoverage = false,
} = {}) {
  const args = ["mcp"];
  if (indexCachePath) {
    args.push("--index-cache", indexCachePath);
  }
  const started = performanceNow();
  const fixture = fixturePath
    ? new WorkbenchMcpSession(loadFixtureManifest(fixturePath))
    : undefined;
  if (fixture) {
    if (!fixture.manifest.allowExistingProcess && fixture.manifest.useProfile) {
      args.push("--workbench-profile-directory", fixture.manifest.profileRoot);
    }
  }
  const client = new McpStdioClient({ serverPath, args });
  let fixtureLaunch;
  let fixtureCleanup;
  let clientInitialized = false;
  let cleanupError;
  let runError;
  let report;
  let consentGuard;
  const fixtureSetupSteps = [];
  try {
    const initialize = await client.initialize();
    clientInitialized = true;
    const listed = await client.listTools();
    const tools = listed?.result?.tools;
    if (!Array.isArray(tools)) {
      throw new Error("MCP tools/list returned no tools array");
    }
    report = {
      kind: "workbench-mcp-corpus",
      corpusVersion: 1,
      statusModel: ["passed", "failed", "blocked", "not-run"],
      server: basename(serverPath),
      protocolVersion: initialize?.result?.protocolVersion ?? null,
      machine: {
        platform: platform(),
        architecture: arch(),
        osRelease: release(),
      },
      elapsedMs: performanceNow() - started,
      contract: buildContractReport({
        apiReference: readFileSync(apiReferencePath, "utf8"),
        listedTools: tools,
      }),
    };
    const endpointPlan = buildWorkbenchEndpointPlan(report.contract.expectedNames);
    report.endpointPlan = endpointPlan;
    report.endpointPlanValidation = validateWorkbenchEndpointPlan(
      report.contract.expectedNames,
      endpointPlan,
      { checkDependencyProducers: true, requireEvidenceSchema: true },
    );
    if (!report.endpointPlanValidation.ok) {
      throw new Error(
        "Workbench endpoint plan is incomplete: " +
          JSON.stringify(report.endpointPlanValidation),
      );
    }
    if (fixture) {
      fixtureLaunch = await fixture.start(client);
      fixtureSetupSteps.push({
        name: "launch-owned-fixture",
        tool: "workbench_launch",
        role: "setup",
        case: "success",
        outcome: "success",
        facts: ["ownedProcess"],
      });
    }
    if (fixture) {
      const readiness = await waitForWorkbenchReady(
        client,
        fixture.manifest.readiness,
      );
      fixtureSetupSteps.push({
        name: "wait-for-workbench",
        tool: "workbench_status",
        role: "setup",
        case: "success",
        outcome: "success",
        facts: ["connectedWorkbench"],
      });
      const worldOpen = await openFixtureWorld(
        client,
        fixture.manifest.expected.worldResource,
        fixture.manifest.readiness,
      );
      const state = worldOpen.state;
      const projectResponse = await client.callTool("workbench_project_context", {});
      const project = projectResponse?.result?.structuredContent;
      const expectedLoadedAddons = fixture.manifest.expected.loadedAddonIds ?? [];
      if (
        projectResponse?.result?.isError === true ||
        !expectedLoadedAddons.every((addonId) => project?.loadedAddons?.includes(addonId))
      ) {
        throw new Error("Fixture loaded addon identities did not match the manifest");
      }
      report.fixture = {
        name: fixture.manifest.name,
        revision: fixture.manifest.revision,
        expected: fixture.manifest.expected,
        processId: fixtureLaunch.processId,
        readiness,
        editor: worldOpen.editor,
        openedEditor: worldOpen.openedEditor,
        openedResource: worldOpen.openedResource,
        activeWorldPath: state.activeWorldPath,
        bridgeVersion: state.bridgeVersion ?? null,
        bridgeProtocolVersion: state.protocolVersion ?? null,
        loadedAddons: project.loadedAddons ?? [],
      };
      fixtureSetupSteps.push(
        {
          name: "discover-world-editor",
          tool: "workbench_list_editors",
          role: "setup",
          case: "success",
          outcome: "success",
          facts: ["worldEditor"],
        },
        {
          name: "open-world-editor",
          tool: "workbench_open_editor",
          role: "setup",
          case: "success",
          outcome: "success",
          facts: ["worldEditor"],
        },
        {
          name: "open-fixture-world",
          tool: "workbench_open_resource",
          role: "setup",
          case: "success",
          outcome: "success",
          facts: ["activeWorld"],
        },
        {
          name: "read-fixture-state",
          tool: "workbench_state",
          role: "setup",
          case: "success",
          outcome: "success",
          facts: ["activeWorld"],
        },
        {
          name: "read-project-context",
          tool: "workbench_project_context",
          role: "setup",
          case: "success",
          outcome: "success",
          facts: ["projectContext"],
        },
      );
    }
    if (fixture && scenarioPath) {
      if (!fixture.manifest.consentGuardProfileRoot) {
        throw new Error(
          "Workbench corpus fixture must define consentGuardProfileRoot for the no-consent bridge case",
        );
      }
      consentGuard = await runConsentGuardProbe({
        serverPath,
        profileRoot: fixture.manifest.consentGuardProfileRoot,
      });
      report.consentGuard = consentGuard;
    }
    const scenarios = scenarioPath
      ? JSON.parse(readFileSync(scenarioPath, "utf8"))
      : undefined;
    if (scenarios) {
      const corpusRunId = `${Date.now()}-${process.pid}`;
      const scenarioContext = fixture
        ? {
            fixture: {
              processId: fixtureLaunch.processId,
              projectPath: fixture.manifest.projectPath,
              worldResource: fixture.manifest.expected.worldResource,
              prefabDestination: `Prefabs/McpConformanceEntity-${corpusRunId}.et`,
              prefabName: `McpConformanceEntity-${corpusRunId}`,
              genericPrefabDestination: `Prefabs/McpConformanceGeneric-${corpusRunId}.et`,
              genericPrefabName: `McpConformanceGeneric-${corpusRunId}`,
              entityName: `McpConformanceEntity-${corpusRunId}`,
              duplicateName: `McpConformanceDuplicate-${corpusRunId}`,
              polylineName: `McpConformancePolyline-${corpusRunId}`,
            },
          }
        : {};
      const definitions = Array.isArray(scenarios.scenarios)
        ? scenarios.scenarios
        : [scenarios];
      const runs = [];
      for (const scenario of definitions) {
        const workflowRun = await runWorkbenchWorkflows({
          client,
          steps: scenario.steps,
          plan: endpointPlan,
          iterations: scenario.iterations,
          warmup: scenario.warmup,
          context: scenarioContext,
          availableFacts: [
            "catalogue",
            ...fixtureSetupSteps.flatMap((step) => step.facts ?? []),
          ],
        });
        runs.push(...workflowRun.runs.map((run) => ({
          ...run,
          scenario: scenario.name ?? null,
        })));
      }
      report.scenarios = [
        ...(consentGuard ? [{ ...consentGuard, scenario: null }] : []),
        ...runs,
      ];
      if (report.scenarios.length === 1) {
        report.scenario = report.scenarios[0];
      }
      report.performance = summarizePerformance(report.scenarios);
      report.endpointCorpus = buildWorkbenchCorpusReport(
        endpointPlan,
        [
          { name: "fixture-setup", steps: fixtureSetupSteps },
          ...(consentGuard ? [{ name: "consent-guard", steps: consentGuard.steps }] : []),
          ...runs,
        ],
      );
      const evidenceTools = new Set(
        report.scenarios.flatMap((run) =>
          run.steps
            .filter(isLiveEvidence)
            .map((step) => step.tool),
        ),
      );
      report.contract.coverage = report.contract.coverage.map((entry) =>
        evidenceTools.has(entry.tool)
          ? { ...entry, liveEvidence: "scenario" }
          : entry,
      );
    } else {
      report.endpointCorpus = buildWorkbenchCorpusReport(
        endpointPlan,
        fixtureSetupSteps.length > 0
          ? [{ name: "fixture-setup", steps: fixtureSetupSteps }]
          : [],
      );
    }
    if (fixture && !fixture.manifest.allowExistingProcess) {
      report.ownedLifecycle = await verifyOwnedStopRestartLifecycle({
        client,
        session: fixture,
      });
      report.endpointCorpus = buildWorkbenchCorpusReport(
        endpointPlan,
        [
          ...(fixtureSetupSteps.length > 0
            ? [{ name: "fixture-setup", steps: fixtureSetupSteps }]
            : []),
          ...(consentGuard ? [{ name: "consent-guard", steps: consentGuard.steps }] : []),
          ...(report.scenarios ?? []),
          { name: "owned-lifecycle", steps: report.ownedLifecycle.invocations },
        ],
      );
    }
  } catch (error) {
    runError = error;
    if (report) {
      report.error = {
        message: error instanceof Error ? error.message : String(error),
        workbench: error?.workbench ?? null,
      };
      const startupSteps = error?.workbench?.phase === "launch"
        ? [{
            name: "fixture-launch",
            tool: "workbench_launch",
            role: "test",
            case: "success",
            outcome: "expected-unavailable",
            blockedBy: ["connectedWorkbench"],
            error: error.workbench,
          }]
        : [];
      report.endpointCorpus ??= buildWorkbenchCorpusReport(
        report.endpointPlan,
        startupSteps.length > 0 ? [{ name: "startup", steps: startupSteps }] : [],
        { blockedFacts: ["ownedProcess", "connectedWorkbench", "managedBridge", "projectContext", "worldEditor", "activeWorld"] },
      );
    }
  } finally {
    try {
      if (fixture) {
        fixtureCleanup = await fixture.stop(clientInitialized ? client : undefined);
      }
    } catch (error) {
      cleanupError = error;
    }
    try {
      await client.close();
    } catch (error) {
      cleanupError ??= error;
    }
  }
  if (cleanupError) {
    if (report) {
      report.error = {
        message: cleanupError instanceof Error ? cleanupError.message : String(cleanupError),
        workbench: cleanupError?.workbench ?? null,
      };
    } else {
      throw cleanupError;
    }
  }
  if (report?.fixture && fixtureCleanup) {
    report.fixture.cleanup = fixtureCleanup;
  }
  if (!report) {
    throw runError ?? new Error("Workbench MCP corpus produced no report");
  }
  report.cleanup = cleanupError
    ? {
        status: "failed",
        error: cleanupError instanceof Error ? cleanupError.message : String(cleanupError),
      }
    : {
        status: fixtureCleanup?.outcome ?? (fixture ? "completed" : "not-required"),
        evidence: fixtureCleanup ?? null,
      };
  if (cleanupError && report.endpointCorpus) {
    report.endpointCorpus.ok = false;
    report.endpointCorpus.cleanup = report.cleanup;
    for (const endpoint of report.endpointCorpus.endpoints) {
      if (endpoint.status === "passed") endpoint.status = "failed";
      endpoint.blockers.push("cleanup:fixture");
    }
    report.endpointCorpus.counts = Object.fromEntries(
      ["passed", "failed", "blocked", "not-run"].map((status) => [
        status,
        report.endpointCorpus.endpoints.filter((endpoint) => endpoint.status === status).length,
      ]),
    );
    report.endpointCorpus.passed = report.endpointCorpus.endpoints
      .filter((endpoint) => endpoint.status === "passed").map((endpoint) => endpoint.tool);
    report.endpointCorpus.failed = report.endpointCorpus.endpoints
      .filter((endpoint) => endpoint.status === "failed").map((endpoint) => endpoint.tool);
  }
  mkdirSync(dirname(reportPath), { recursive: true });
  writeFileSync(reportPath, JSON.stringify(report, null, 2) + "\n", "utf8");
  return report;
}

export function resolveServerPath(explicitPath) {
  if (explicitPath) {
    if (!existsSync(explicitPath)) {
      throw new Error("MCP server does not exist: " + explicitPath);
    }
    return resolve(explicitPath);
  }
  const candidate = defaultServerCandidates.find((path) => existsSync(path));
  if (!candidate) {
    throw new Error(
      "No bundled MCP server found. Build it first or pass --server <path>.",
    );
  }
  return candidate;
}

function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function fingerprint(value) {
  return createHash("sha256")
    .update(typeof value === "string" ? value : JSON.stringify(value))
    .digest("hex");
}

function listRelativeEntries(root) {
  if (!existsSync(root)) return [];
  const entries = [];
  const visit = (current, prefix) => {
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const relativeEntry = prefix ? join(prefix, entry.name) : entry.name;
      const absoluteEntry = join(current, entry.name);
      entries.push(
        entry.isDirectory()
          ? relativeEntry + ":directory"
          : relativeEntry + ":file:" + createHash("sha256").update(readFileSync(absoluteEntry)).digest("hex"),
      );
      if (entry.isDirectory()) {
        visit(absoluteEntry, relativeEntry);
      }
    }
  };
  visit(root, "");
  return entries.sort();
}

function isWithin(root, child) {
  const relativePath = relative(resolve(root), resolve(child));
  return (
    relativePath === "" ||
    (!relativePath.startsWith("..") && !isAbsolute(relativePath))
  );
}

function performanceNow() {
  return Number(process.hrtime.bigint() / 1000000n);
}

function readJsonPointer(value, pointer) {
  if (pointer === "") {
    return value;
  }
  if (!pointer.startsWith("/")) {
    return undefined;
  }
  return pointer
    .slice(1)
    .split("/")
    .map((part) => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((current, part) => current?.[part], value);
}

function materialize(value, context) {
  if (Array.isArray(value)) {
    return value.map((item) => materialize(item, context));
  }
  if (isObject(value)) {
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => [key, materialize(child, context)]),
    );
  }
  if (typeof value !== "string" || !value.includes("$")) {
    return value;
  }
  const resolveReference = (reference) => {
    let current = context;
    for (const part of reference.split(".")) {
      current = current?.[part];
    }
    return current;
  };
  const exactReference = /^\$([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)$/.exec(value);
  if (exactReference) {
    const resolved = resolveReference(exactReference[1]);
    return resolved === undefined ? value : resolved;
  }
  return value.replace(/\$([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)/g, (match, reference) => {
    const resolved = resolveReference(reference);
    return resolved === undefined ? match : String(resolved);
  });
}

function makeDistinctValue(value) {
  if (typeof value === "number") return value === 0 ? 1 : value + 1;
  if (typeof value === "boolean") return !value;
  if (typeof value === "string") return value + "-changed";
  if (Array.isArray(value)) {
    return value.length === 0
      ? [1]
      : [makeDistinctValue(value[0]), ...value.slice(1)];
  }
  if (isObject(value)) {
    if (["x", "y", "z"].every((key) => typeof value[key] === "number")) {
      return { ...value, x: makeDistinctValue(value.x) };
    }
    const primitiveKey = Object.keys(value).find((key) =>
      ["number", "boolean", "string"].includes(typeof value[key]),
    );
    return primitiveKey
      ? { ...value, [primitiveKey]: makeDistinctValue(value[primitiveKey]) }
      : value;
  }
  return value === null || value === undefined ? 1 : value;
}

function waitForClose(child) {
  if (child.exitCode !== null) {
    return Promise.resolve();
  }
  return new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error("MCP process did not exit after stdin closed"));
    }, 5000);
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.once("close", () => {
      clearTimeout(timer);
      resolvePromise();
    });
  });
}

function waitForCloseWithin(child, timeoutMs) {
  if (child.exitCode !== null) {
    return Promise.resolve();
  }
  return new Promise((resolvePromise, reject) => {
    const timer = setTimeout(() => {
      reject(new Error("process did not exit before cleanup deadline"));
    }, timeoutMs);
    child.once("error", (error) => {
      clearTimeout(timer);
      reject(error);
    });
    child.once("close", () => {
      clearTimeout(timer);
      resolvePromise();
    });
  });
}

function delay(milliseconds) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds));
}

function parseArguments(argumentsList) {
  const options = {};
  for (let index = 0; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--help" || argument === "-h") {
      options.help = true;
      continue;
    }
    if (argument === "--require-live-coverage") {
      options.requirelivecoverage = true;
      continue;
    }
    if (
      ![
        "--server",
        "--api-reference",
        "--out",
        "--index-cache",
        "--scenario",
        "--fixture",
      ].includes(argument)
    ) {
      throw new Error("Unknown argument: " + argument);
    }
    const value = argumentsList[index + 1];
    if (!value || value.startsWith("--")) {
      throw new Error(argument + " requires a value");
    }
    options[argument.slice(2).replaceAll("-", "")] = value;
    index += 1;
  }
  return options;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const options = parseArguments(process.argv.slice(2));
    if (options.help) {
      console.log(
        "Usage: node tools/workbench-mcp-conformance.mjs [--server PATH] " +
          "[--api-reference PATH] [--index-cache PATH] [--scenario PATH] [--fixture PATH] " +
          "[--require-live-coverage] [--out PATH]",
      );
      process.exit(0);
    }
    if (options.scenario && !options.fixture) {
      throw new Error("--scenario requires --fixture so live operations are disposable");
    }
    const report = await runWorkbenchCorpus({
      serverPath: resolveServerPath(options.server),
      apiReferencePath: options.apireference ?? defaultApiReference,
      indexCachePath: options.indexcache,
      scenarioPath: options.scenario,
      fixturePath: options.fixture,
      requireLiveCoverage: options.requirelivecoverage === true,
      reportPath: options.out ?? defaultReportPath,
    });
    console.log(JSON.stringify(report, null, 2));
    const corpusRequired = Boolean(options.scenario) || options.requirelivecoverage === true;
    process.exit(
      !report.error &&
        report.contract.ok &&
        (!report.scenarios || report.scenarios.every((scenario) => scenario.ok)) &&
        (!corpusRequired || report.endpointCorpus?.ok)
        ? 0
        : 1,
    );
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
