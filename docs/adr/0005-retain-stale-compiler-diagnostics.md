# Retain stale compiler diagnostics

When a newer relevant script change follows a Workbench validation request,
the resulting Compiler Diagnostics remain visible as explicitly labelled
prior-snapshot evidence until a fresh result replaces them.  This preserves
useful Workbench feedback without representing it as current-buffer truth.
