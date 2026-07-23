# Preserve compiler diagnostic provenance

Workbench Compiler Diagnostics and Provisional Parser Diagnostics remain
separate sources.  A completed Workbench validation is authoritative for its
saved configuration snapshot, while Rust analysis continues to serve unsaved
editing and unavailable-validation states without being presented as equivalent
compiler truth.
