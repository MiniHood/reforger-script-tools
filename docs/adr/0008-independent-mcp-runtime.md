# Keep the MCP runtime independent from the LSP process

The MCP runtime is a separately launched Rust process that reuses the same
language-engine modules and validated game-data disk cache as the LSP. It loads
game data lazily and does not proxy through or attach to the editor-owned LSP,
preserving MCP availability when VS Code or the language server is not running.
A shared engine daemon is justified only if measurements show that duplicate
runtime memory or warm-cache reconstruction outweighs the added lifecycle and
coordination complexity.
