# Host-neutral Workbench Gateway

The extension and a future MCP host will share one host-neutral Workbench
Gateway for the private Workbench NET API.  The Gateway exposes named, typed
Workbench Capabilities rather than arbitrary endpoint dispatch; the extension
hosts it initially for compiler validation, and a future MCP host adapts the
same boundary instead of creating a second NET API implementation.
