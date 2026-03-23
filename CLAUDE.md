# quantum-sdk-rust

## SDK Parity Check

This SDK must stay in sync with the Rust reference SDK. Use `sdk-graph` to check parity:

```bash
# Scan this SDK (run after making changes)
sdk-graph scan --sdk rust --dir ~/work/tauri_apps/qe-sdk-collection/rust_projects/quantum-sdk/src

# Show overall stats
sdk-graph stats
```

Binary: `~/go/bin/sdk-graph` (in PATH)
Graph file: `~/work/go_programs/quantum-ai/sdk-graph.json` (shared across all SDKs)

## Workflow

1. After adding or modifying types: rescan with `sdk-graph scan --sdk rust --dir ~/work/tauri_apps/qe-sdk-collection/rust_projects/quantum-sdk/src`
2. Check downstream SDKs: run `sdk-graph diff --base rust --target go`, `--target ts`, `--target python`
3. Goal: zero missing types and fields in all downstream SDKs vs Rust

## Reference Implementation

This is the reference SDK. When adding new API types, other SDKs will sync from here.

## API Server

Backend: https://api.quantumencoding.ai
Repo: ~/work/go_programs/quantum-ai
