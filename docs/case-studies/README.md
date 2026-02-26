# Case Studies

Validation of `codehud` against real-world open source repositories across major languages. Each case study exercises codehud's features against a large, structurally complex codebase to surface bugs, performance issues, and missing coverage.

## Why

Agents need code intelligence that works on *real* repos — not toy examples. These case studies prove (or disprove) that codehud delivers useful structural context at scale.

## Status

| Language | Repo | Status |
|---|---|---|
| **TypeScript** | [microsoft/vscode](vscode.md) | 🟡 In progress |
| **TypeScript** | typeorm/typeorm | ⬜ Not started |
| **TypeScript** | angular/angular | ⬜ Not started |
| **JavaScript** | expressjs/express | ⬜ Not started |
| **JavaScript** | mrdoob/three.js | ⬜ Not started |
| **JavaScript** | facebook/react | ⬜ Not started |
| **Rust** | servo/servo | ⬜ Not started |
| **Rust** | rust-lang/rust-analyzer | ⬜ Not started |
| **Rust** | tokio-rs/tokio | ⬜ Not started |
| **Kotlin** | android/nowinandroid | ⬜ Not started |
| **Kotlin** | JetBrains/kotlin | ⬜ Not started |
| **Kotlin** | square/okhttp | ⬜ Not started |
| **C#** | dotnet/runtime | ⬜ Not started |
| **C#** | dotnet/roslyn | ⬜ Not started |
| **C#** | abpframework/abp | ⬜ Not started |
| **Java** | spring-projects/spring-framework | ⬜ Not started |
| **Java** | elastic/elasticsearch | ⬜ Not started |
| **Java** | apache/kafka | ⬜ Not started |
| **Go** | kubernetes/kubernetes | ⬜ Not started |
| **Go** | hashicorp/terraform | ⬜ Not started |
| **Go** | prometheus/prometheus | ⬜ Not started |

## Standard Test Matrix

Every case study should exercise:

- `--stats` — repo profile
- `--outline` — structural overview
- `--list-symbols` — symbol extraction
- `--search` — pattern matching with context
- `--xrefs` — cross-file reference tracing
- `--diff` — structural diff
- `--smart-depth` — monorepo handling

See [issue #29](https://github.com/Tidemarks-AI/Code-HUD/issues/29) for the full spec.
