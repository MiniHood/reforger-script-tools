# Default continuous compiler validation

Continuous Compiler Validation is enabled by default with a three-second idle
delay, and the same single delay setting permits manual-only validation as an
explicit opt-out.  This prioritizes timely Workbench compiler feedback while
coalescing editing bursts into one validation request. An idle validation saves
only its active Validation Save Target, never all dirty script documents.
