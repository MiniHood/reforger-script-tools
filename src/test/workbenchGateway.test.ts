import * as assert from "node:assert";
import {
  WorkbenchGateway,
  workbenchLogReportsMissingHandler,
} from "../workbenchNetApi/gateway/workbenchGateway";
import { encodeNetApiString, startNetApiPeer } from "./netApiPeer";

suite("Workbench Gateway", () => {
  test("matches generic missing NET API log evidence", () => {
    const lines = [
      "01:22:48.623 NETWORK   (E): Failed to call not existing Net API function 'RST_WorkbenchOtherOperation'",
      "01:23:05.325 NETWORK   (E): Failed to call not existing Net API function 'RST_WorkbenchOtherOperation'",
    ];

    assert.strictEqual(
      workbenchLogReportsMissingHandler(
        lines,
        "RST_WorkbenchOtherOperation",
      ),
      true,
    );
    assert.strictEqual(
      workbenchLogReportsMissingHandler(lines, "RST_WorkbenchMissingOperation"),
      false,
    );
    assert.strictEqual(workbenchLogReportsMissingHandler(lines), true);
  });

  test("diagnoses a failed status call from a live process and generic log evidence", async () => {
    const gateway = new WorkbenchGateway({
      enabled: true,
      endpoint: { host: "127.0.0.1", port: 5775 },
    });
    gateway.getProcessStatus = async () => ({
      ok: true,
      value: { isOpen: true },
    });
    gateway.readWorkbenchLogs = async () => ({
      ok: true,
      value: {
        source: "workbench",
        lines: [
          "NETWORK (E): Failed to call not existing Net API function 'RST_WorkbenchOtherOperation'",
        ],
        markers: [],
        truncated: false,
      },
    });

    assert.strictEqual(
      await gateway.diagnoseNetApiFailure(undefined, {
        ok: false,
        failure: { category: "unavailable", recoveryHint: "retry" },
      }),
      "scripts-failing",
    );
  });

  test("gets compiler readiness through the documented NET API framing", async () => {
    const peer = await startNetApiPeer((request) => {
      assert.strictEqual(request.protocolVersion, 1);
      assert.strictEqual(request.clientId, "ReforgerScriptTools");
      assert.strictEqual(request.contentType, "JsonRPC");
      assert.deepStrictEqual(request.payload, {
        APIFunc: "IsWorkbenchRunning",
      });
      return {
        errorCode: "Ok",
        payload: { IsRunning: true, ScriptsCompiled: true },
      };
    });
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
      });

      assert.deepStrictEqual(await gateway.getStatus(), {
        ok: true,
        value: { isRunning: true, scriptsCompiled: true },
      });
      assert.deepStrictEqual(gateway.availability, { kind: "ready" });
    } finally {
      await peer.close();
    }
  });

  test("validates the named WORKBENCH profile and normalizes compiler diagnostics", async () => {
    const peer = await startNetApiPeer((request) => {
      assert.deepStrictEqual(request.payload, {
        APIFunc: "ValidateScripts",
        Configuration: "WORKBENCH",
      });
      return {
        errorCode: "Ok",
        payload: {
          Errors: [
            {
              error: "Broken expression (missing ';'?)",
              file: "scripts/Game/Example.c",
              fileAbs: "C:\\Addon\\scripts\\Game\\Example.c",
              addon: "ExampleAddon",
              line: 12,
            },
            {
              error: "Broken expression (missing ';'?)",
              file: "scripts/Game/Example.c",
              fileAbs: "C:\\Addon\\scripts\\Game\\Example.c",
              addon: "ExampleAddon",
              line: 12,
            },
            {
              error: "Assign operator '=' not allowed here",
              file: "scripts/Game/Example.c",
              fileAbs: "C:\\Addon\\scripts\\Game\\Example.c",
              addon: "ExampleAddon",
              line: 12,
            },
          ],
          Warnings: [
            {
              error: "Broken expression (missing ';'?)",
              file: "scripts/Game/Example.c",
              fileAbs: "C:\\Addon\\scripts\\Game\\Example.c",
              addon: "ExampleAddon",
              line: 12,
            },
            {
              error: "Variable 'unused' is not used",
              file: "scripts/Game/Other.c",
              line: 4,
            },
          ],
          Success: false,
        },
      };
    });
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
      });

      assert.deepStrictEqual(await gateway.validateScripts("WORKBENCH"), {
        ok: true,
        value: {
          profile: "WORKBENCH",
          success: false,
          diagnostics: [
            {
              severity: "error",
              message: "Broken expression (missing ';'?)",
              location: {
                file: "scripts/Game/Example.c",
                fileAbs: "C:\\Addon\\scripts\\Game\\Example.c",
                addon: "ExampleAddon",
                line: 12,
              },
            },
            {
              severity: "error",
              message: "Assign operator '=' not allowed here",
              location: {
                file: "scripts/Game/Example.c",
                fileAbs: "C:\\Addon\\scripts\\Game\\Example.c",
                addon: "ExampleAddon",
                line: 12,
              },
            },
            {
              severity: "warning",
              message: "Variable 'unused' is not used",
              location: {
                file: "scripts/Game/Other.c",
                line: 4,
              },
            },
          ],
        },
      });
    } finally {
      await peer.close();
    }
  });

  test("reports only a sanitized named-capability outcome to its host", async () => {
    const records: unknown[] = [];
    const peer = await startNetApiPeer(() => ({
      errorCode: "Ok",
      payload: { IsRunning: true, ScriptsCompiled: true },
    }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
        record: (record) => records.push(record),
      });

      await gateway.getStatus();

      assert.strictEqual(records.length, 1);
      assert.deepStrictEqual(records[0], {
        capability: "getStatus",
        outcome: "success",
        durationMs: (records[0] as { durationMs: number }).durationMs,
        timing: (records[0] as { timing: unknown }).timing,
      });
      assert.ok(
        Number.isFinite((records[0] as { durationMs: number }).durationMs),
      );
      const timing = (
        records[0] as {
          timing?: { callbackMs?: number; request?: { totalMs?: number } };
        }
      ).timing;
      assert.ok(Number.isFinite(timing?.callbackMs));
      assert.ok(Number.isFinite(timing?.request?.totalMs));
      const serialized = JSON.stringify(records[0]);
      assert.ok(!serialized.includes(String(peer.port)));
      assert.ok(!serialized.includes("IsWorkbenchRunning"));
      assert.ok(!serialized.includes("127.0.0.1"));
    } finally {
      await peer.close();
    }
  });

  test("rejects a non-loopback endpoint without network discovery", async () => {
    const gateway = new WorkbenchGateway({
      enabled: true,
      endpoint: { host: "192.0.2.10", port: 5775 },
    });

    assert.deepStrictEqual(await gateway.getStatus(), {
      ok: false,
      failure: {
        category: "unsupported",
        recoveryHint: "Configure a loopback Workbench host such as 127.0.0.1.",
      },
    });
  });

  test("performs no transaction while the Gateway is disabled", async () => {
    const gateway = new WorkbenchGateway({
      enabled: false,
      endpoint: { host: "127.0.0.1", port: 1 },
    });

    const result = await gateway.getStatus();

    assert.strictEqual(result.ok, false);
    assert.deepStrictEqual(gateway.availability, { kind: "disabled" });
  });

  test("rejects an unnamed validation profile before opening a connection", async () => {
    const gateway = new WorkbenchGateway({
      enabled: true,
      endpoint: { host: "127.0.0.1", port: 1 },
    });

    assert.deepStrictEqual(await gateway.validateScripts("PC" as never), {
      ok: false,
      failure: {
        category: "unsupported",
        recoveryHint: "Select the supported WORKBENCH validation profile.",
      },
    });
  });

  test("categorizes a Workbench error code separately from compiler findings", async () => {
    const peer = await startNetApiPeer(() => ({
      errorCode: "InvalidRequest",
      payload: {},
    }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
      });

      const result = await gateway.validateScripts("WORKBENCH");

      assert.strictEqual(result.ok, false);
      if (!result.ok) {
        assert.strictEqual(result.failure.category, "workbench-error");
      }
    } finally {
      await peer.close();
    }
  });

  test("categorizes a truncated response as a protocol failure", async () => {
    const peer = await startNetApiPeer(() => ({
      errorCode: "Ok",
      payload: {},
      raw: encodeNetApiString("Ok"),
    }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
      });

      const result = await gateway.getStatus();

      assert.strictEqual(result.ok, false);
      if (!result.ok) {
        assert.strictEqual(result.failure.category, "protocol");
      }
    } finally {
      await peer.close();
    }
  });

  test("categorizes malformed JSON as a protocol failure", async () => {
    const peer = await startNetApiPeer(() => ({
      errorCode: "Ok",
      payload: {},
      raw: Buffer.concat([
        encodeNetApiString("Ok"),
        encodeNetApiString("{not-json"),
      ]),
    }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
      });

      const result = await gateway.getStatus();

      assert.strictEqual(result.ok, false);
      if (!result.ok) {
        assert.strictEqual(result.failure.category, "protocol");
      }
    } finally {
      await peer.close();
    }
  });

  test("reports the Workbench API ready even when scripts did not compile", async () => {
    const peer = await startNetApiPeer(() => ({
      errorCode: "Ok",
      payload: { IsRunning: true, ScriptsCompiled: false },
    }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
      });

      assert.strictEqual((await gateway.getStatus()).ok, true);
      assert.deepStrictEqual(gateway.availability, { kind: "ready" });
    } finally {
      await peer.close();
    }
  });

  test("categorizes an unresponsive endpoint as a timeout", async () => {
    const peer = await startNetApiPeer(() => ({ silent: true }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
        deadlines: { getStatusMs: 30 },
      });

      const result = await gateway.getStatus();

      assert.strictEqual(result.ok, false);
      if (!result.ok) {
        assert.strictEqual(result.failure.category, "timeout");
      }
    } finally {
      await peer.close();
    }
  });

  test("enforces a wall-clock deadline while a peer trickles response bytes", async () => {
    const peer = await startNetApiPeer(() => ({
      rawChunks: [
        Buffer.from([20]),
        Buffer.from([0]),
        Buffer.from([0]),
        Buffer.from([0]),
      ],
      intervalMs: 20,
    }));
    try {
      const gateway = new WorkbenchGateway({
        enabled: true,
        endpoint: { host: "127.0.0.1", port: peer.port },
        deadlines: { getStatusMs: 35 },
      });
      const result = await gateway.getStatus();

      assert.strictEqual(result.ok, false);
      if (!result.ok) {
        assert.strictEqual(result.failure.category, "timeout");
      }
    } finally {
      await peer.close();
    }
  });

  test("categorizes a refused configured endpoint as unavailable", async () => {
    const peer = await startNetApiPeer(() => ({ silent: true }));
    const port = peer.port;
    await peer.close();
    const gateway = new WorkbenchGateway({
      enabled: true,
      endpoint: { host: "127.0.0.1", port },
      deadlines: { getStatusMs: 100 },
    });

    const result = await gateway.getStatus();

    assert.strictEqual(result.ok, false);
    if (!result.ok) {
      assert.strictEqual(result.failure.category, "unavailable");
    }
  });
});
