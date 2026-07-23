# Use an explicitly configured Workbench endpoint

The Workbench Gateway uses the extension-owned Configured Workbench Endpoint,
with loopback-only host default `127.0.0.1` and port default `5775`, rather
than discovering or scanning for a NET API listener. This keeps connection
authority with the user and avoids silently selecting an unintended Workbench
process.
