# src/diagnostics/diagnostics.ts

## Purpose

Provides the extension-host diagnostic performance stream. It writes bounded,
local JSONL records so support investigations can reconstruct extension
lifecycle and client activity without using the VS Code output channel as a
data store.

## Ownership

This module owns extension-host diagnostic file preparation, retention, session
identity, and serialized asynchronous writes. Callers own the meaning and safe
scalar fields of their events. Rust owns the separate language-server stream.

## Current Behavior

Diagnostics are enabled by default through
`reforgerScriptTools.diagnostics.enabled`. When enabled, the module creates
`globalStorageUri/logs/extension-diagnostics.jsonl`, retains bounded recent
history, and queues append writes behind one promise chain. Each record has a
timestamp, `extension` component, session identifier, event, and safe scalar
fields. Failed diagnostic file operations are ignored so they cannot disrupt
editor work.

The module does not log source text, LSP payloads, completion/hover content,
or arbitrary objects. The language client passes a separate sibling path to
the Rust server only when diagnostics are enabled.

## Dependencies and Boundaries

Depends on VS Code configuration and Node file APIs only. It must not inspect
language syntax, LSP request bodies, game data, or editor feature results.
Runtime files remain in global storage and never in a workspace.

## Verification

Run `npm test` to verify extension activation and the default setting, and
inspect a local extension-host session to confirm the JSONL stream exists.
Run `cargo test` to verify the independent Rust stream's structured output.
