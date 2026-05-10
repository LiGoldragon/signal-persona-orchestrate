# skills — signal-persona-mind

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
- `~/primary/reports/operator/100-persona-mind-central-rename-plan.md`
  — the design report grounding this channel.
- this repo's `ARCHITECTURE.md`.
- the consumer's `ARCHITECTURE.md` (`persona-mind/`).

If your change adds a new request or reply variant, edit
`src/lib.rs` first, push, then update the consumer
(`persona-mind`) to handle it.

## What this repo owns

- `RoleName` (closed enum: Operator, OperatorAssistant,
  Designer, DesignerAssistant, SystemSpecialist, Poet).
- `ScopeReference` (closed enum: Path | Task) plus
  `WirePath` and `TaskToken` newtypes.
- `ScopeReason` (provisional `String` newtype).
- `TimestampNanos` (store-supplied; never agent-supplied).
- The closed `MindRequest` enum (`RoleClaim`,
  `RoleRelease`, `RoleHandoff`, `RoleObservation`,
  `ActivitySubmission`, `ActivityQuery`, `Open`,
  `AddNote`, `Link`, `ChangeStatus`, `AddAlias`, `Query`).
- The closed `MindReply` enum (`ClaimAcceptance`,
  `ClaimRejection`, `ReleaseAcknowledgment`,
  `HandoffAcceptance`, `HandoffRejection`, `RoleSnapshot`,
  `ActivityAcknowledgment`, `ActivityList`, `Opened`,
  `NoteAdded`, `Linked`, `StatusChanged`, `AliasAdded`,
  `View`, `Rejected`).
- The mind memory/work record vocabulary: `Item`, `Note`, `Edge`,
  `Event`, aliases, references, and ready-query records.
- The `Frame` type alias and round-trip tests.

## What this repo does not own

- The state actor or the database — that's
  `persona-mind`.
- The CLI binary parsing — that's the `orchestrate` bin
  compatibility shim and the `mind` bin target inside `persona-mind`.
- Lock-file projection writing — that's
  `persona-mind`.
- The activity log retention policy — that's
  `persona-mind`.
- Storage tables — those live in `persona-mind`'s
  `src/tables.rs` (typed `sema::Table<K, V>` constants).
