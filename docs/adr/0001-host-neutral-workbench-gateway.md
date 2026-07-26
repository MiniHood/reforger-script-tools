# Host-neutral Workbench Gateway

The extension and any MCP Workbench adapter share one host-neutral Workbench
Gateway for the private Workbench NET API.  The Gateway exposes named, typed
Workbench Capabilities rather than arbitrary endpoint dispatch; the extension
hosts it for compiler validation, and an MCP Workbench adapter adapts the
same boundary instead of creating a second NET API implementation.
