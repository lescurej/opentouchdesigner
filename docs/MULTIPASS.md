# Multi-pass GPU (status)

Today:

- **Feedback TOP** — double-buffered; reads the previous cooked frame of its target.
- **Cache TOP** — holds a persistent texture until reset.
- **Select TOP** — samples inputs at cook time (this frame).

Planned (not fully wired):

- Named render targets per pass for ISF-style multi-pass shaders.
- Persistent buffers shared across passes for Analyze-style ops and GPU particles.

Cook semantics for feedback loops are covered by `otd-gpu` integration tests (`feedback_reads_the_previous_frame`, etc.).
