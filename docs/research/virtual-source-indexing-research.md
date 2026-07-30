# Virtual source indexing and navigation research

**Question.** Must the Reforger language server extract PAK scripts to loose
physical files in order to index them and support Go to Definition?

## Finding

No. Source bytes need a stable identity and a way to be supplied when the
editor opens that identity; they do not intrinsically need one OS file per
script. Established tooling uses both models:

- Eclipse JDT and IntelliJ IDEA treat JAR/ZIP archives as library inputs and
  browse/navigate attached source archives directly. Eclipse documents that
  its Package Explorer can browse internal and external JAR contents and shows
  attached source when a Java element in a JAR is opened; its search results
  retain source positions relative to the attachment. IntelliJ accepts JAR and
  ZIP archives as Java library content and classifies their contents as
  sources. [Eclipse Package Explorer](https://help.eclipse.org/latest/topic/org.eclipse.jdt.doc.user/reference/views/ref-view-package-explorer.htm),
  [Eclipse JDT search](https://help.eclipse.org/latest/topic/org.eclipse.jdt.doc.isv/guide/jdt_api_search.htm),
  [IntelliJ libraries](https://www.jetbrains.com/help/idea/library.html).
- `rust-analyzer` makes the distinction explicitly: its VFS is a flat set of
  files identified by interned abstract paths and receives file contents from a
  separate loader; the VFS itself performs neither I/O nor watching. It accepts
  bytes through `set_file_contents` and can mark a file deleted. This is a
  direct example of indexing/navigation identity being independent of a loose
  physical file. [rust-analyzer VFS source](https://github.com/rust-lang/rust-analyzer/blob/master/crates/vfs/src/lib.rs).
- TypeScript's language-service host receives a script-name list, version, and
  `IScriptSnapshot` text abstraction. A snapshot can report a precise change
  range for incremental parsing, or none when full reparse is necessary. The
  API also has optional custom module resolution. The standard `tsserver` host
  normally uses real `node_modules`, but the language engine interface itself
  is not tied to physical source files. [TypeScript host and snapshot source](https://github.com/microsoft/TypeScript/blob/main/src/services/types.ts).
- Go chose the other pragmatic model for downloaded modules: the Go command
  expands source into the shared, read-only `GOMODCACHE`; current `gopls`
  persists an index of packages in that cache. This demonstrates that a
  physical cache is common and robust, not that it is required by LSP.
  [Go module-cache reference](https://go.dev/ref/mod#module-cache),
  [gopls 0.19 release notes](https://go.dev/gopls/release/v0.19.0).

## VS Code and LSP constraints

VS Code supports readonly documents from arbitrary sources through a registered
URI scheme and `TextDocumentContentProvider`; it calls the provider when the
URI is opened, and these documents participate in normal text-document
infrastructure. For a hierarchy of files, a `FileSystemProvider` can expose a
full virtual filesystem. VS Code also cautions extensions not to assume
`uri.fsPath`: documents need not be on disk. [Virtual Documents API](https://code.visualstudio.com/api/extension-guides/virtual-documents),
[Document selectors](https://code.visualstudio.com/api/references/document-selector).

That does **not** make virtual sources free. VS Code's virtual-workspace guide
says its bundled rich language extensions provide only single-file support on
virtual resources, and describes cross-file support as a harder capability;
it also notes that filesystem-provider support in LSP is still under
development. Therefore, our native Rust server must remain able to resolve and
read every indexed PAK entry itself rather than relying on VS Code to make a
virtual PAK tree look like a local workspace. [Virtual Workspaces guide](https://code.visualstudio.com/api/extension-guides/virtual-workspaces).

## Recommended direction for this project

Keep the PAC reader as the authoritative source representation, and index
scripts directly from its selected PAK entries. Persist an add-on manifest
containing the add-on identity/fingerprint, PAK identities, logical script
path, and PAC entry location/compression metadata. Locations returned by
definition can use a stable read-only URI such as:

```
reforger-pak://<addon-guid>/<fingerprint>/scripts/<logical-path>.c
```

The extension can serve that URI through a `TextDocumentContentProvider`, which
asks the Rust server (or a small, explicitly scoped bridge) for exactly the one
entry. This preserves precise per-file editor tabs and definition targets while
eliminating the 6,495 loose-file writes from the first extraction path.

Do **not** make this a blind replacement for physical files yet. Use physical
extraction only if a proven integration needs `file:` URIs (for example, an
external tool that cannot consume document content), and isolate it as an
optional materialization cache. The initial virtual-source slice needs an
end-to-end proof that a `reforger-pak:` definition location opens correctly,
that the document is associated with the Enfusion language, and that the Rust
server can service cross-file navigation without a directory scan. That is the
relevant acceptance test; archive-based Java tooling proves feasibility but not
our exact VS Code/LSP integration.

## Implication for performance

The measured 3.2--3.6 second base-game run is principally the cost of creating
and closing thousands of physical output files. A direct PAC-backed index still
must read/decompress script text once to parse it, but it avoids those output
operations. It should retain a versioned manifest for independent add-on
rebuilds and support on-demand decoding for the comparatively rare editor-open
operation. This is an architectural option to prototype and measure, rather
than a claim that it will always outperform a warm filesystem cache.
