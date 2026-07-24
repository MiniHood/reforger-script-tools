# Default continuous compiler validation

Continuous Compiler Validation uses a fixed three-second idle interval for
unsaved typing. This keeps a stable compiler-feedback policy without exposing
a setting that has no supported alternative. Saving validates immediately, and
an idle validation saves only its active Validation Save Target, never all
dirty script documents.
