# Use a native Enforce semantic palette overlay

The extension uses one dark-oriented Reforger Semantic Palette as default
Enforce-qualified `editor.semanticTokenColorCustomizations` foreground rules
owned by `package.json`, rather than selecting or contributing a complete color
theme. The Rust language engine owns semantic classifications but no colors;
`reforgerField`, `reforgerPunctuation`, and `reforgerPreprocessor` preserve
Reforger-facing vocabulary, while one `function:enforce` palette role colors
both global functions and class methods without erasing their structural
language-model distinction. Users retain their chosen theme and may override
individual rules or disable semantic highlighting through native VS Code
settings; font styles remain theme-owned, the existing bracket modes retain
their behavior, and an official light palette is deferred until it is
deliberately designed and contrast-tested.
