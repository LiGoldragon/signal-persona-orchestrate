# skills — signal-persona-orchestrate

*Per-repo agent guide.*

## Checkpoint — read before editing

Before changing code in this repo, read:

- `~/primary/skills/contract-repo.md` — contract-repo
  discipline (what goes here vs. doesn't).
- `~/primary/skills/architecture-editor.md` — `ARCHITECTURE.md`
  conventions.
- `~/primary/skills/architectural-truth-tests.md` — every
  contract change needs a witness test.
- `~/primary/skills/nix-discipline.md` — flake-input rules,
  `nix flake check` is the gate.
- `~/primary/reports/designer/93-persona-orchestrate-rust-rewrite-and-activity-log.md`
  — the design report grounding this channel.
- this repo's `ARCHITECTURE.md`.
- the consumer's `ARCHITECTURE.md` (`persona-orchestrate/`).

If your change adds a new request or reply variant, edit
`src/lib.rs` first, push, then update the consumer
(`persona-orchestrate`) to handle it.

## What this repo owns

- `RoleName` (closed enum: Operator, Designer,
  SystemSpecialist, Poet, Assistant).
- `ScopeReference` (closed enum: Path | Task) plus
  `WirePath` and `TaskToken` newtypes.
- `ScopeReason` (provisional `String` newtype).
- `TimestampNanos` (store-supplied; never agent-supplied).
- The closed `OrchestrateRequest` enum (`RoleClaim`,
  `RoleRelease`, `RoleHandoff`, `RoleObservation`,
  `ActivitySubmission`, `ActivityQuery`).
- The closed `OrchestrateReply` enum (`ClaimAcceptance`,
  `ClaimRejection`, `ReleaseAcknowledgment`,
  `HandoffAcceptance`, `HandoffRejection`, `RoleSnapshot`,
  `ActivityAcknowledgment`, `ActivityList`).
- The `Frame` type alias and round-trip tests.

## What this repo does not own

- The state actor or the database — that's
  `persona-orchestrate`.
- The CLI binary parsing — that's the `orchestrate` bin
  target inside `persona-orchestrate`.
- Lock-file projection writing — that's
  `persona-orchestrate`.
- The activity log retention policy — that's
  `persona-orchestrate`.
- Storage tables — those live in `persona-orchestrate`'s
  `src/tables.rs` (typed `sema::Table<K, V>` constants).
