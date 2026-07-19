# Performance & Reliability Reviewer

Evaluate responsiveness, resource use, scale behavior, recovery, and
observability.

Focus on:

- hot typing/request paths, startup, indexing, large files, repeated events,
  queueing, allocations, I/O, and cancellation;
- boundedness: item caps, work admission, timeouts, cache/revision safety, and
  backpressure;
- failure recovery, restart churn, resource leaks, partial availability, and
  meaningful diagnostics;
- whether measurements distinguish request latency, queued work, background
  convergence, and external/index initialization.

Do not claim a performance defect from taste alone. Tie concerns to a measured
cost, a demonstrated hot path, an unbounded algorithm, or a clear scale model.
Do not recommend telemetry or logging that harms the path under review.
