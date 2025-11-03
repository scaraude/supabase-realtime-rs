---
name: Bug Report
about: Report a bug or unexpected behavior
title: '[BUG] '
labels: bug
assignees: ''
---

## Describe the Bug

A clear and concise description of what the bug is.

## To Reproduce

Steps to reproduce the behavior:

1. Create a client with '...'
2. Subscribe to channel '...'
3. Call method '...'
4. See error

## Expected Behavior

A clear and concise description of what you expected to happen.

## Actual Behavior

What actually happened instead.

## Code Sample

```rust
// Minimal reproducible example
use supabase_realtime_rs::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Your code here
    Ok(())
}
```

## Environment

- **Rust Version**: (output of `rustc --version`)
- **supabase-realtime-rs Version**: (e.g., 0.1.0, git commit hash)
- **OS**: (e.g., macOS 14.0, Ubuntu 22.04, Windows 11)
- **Supabase Project**: (self-hosted or cloud, version if known)

## Error Output

```
Paste any error messages or stack traces here
```

## Additional Context

Add any other context about the problem here (screenshots, logs, related issues, etc.)

## Possible Solution

(Optional) If you have ideas on how to fix this, describe them here.
