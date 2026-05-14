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
- this repo's `ARCHITECTURE.md`.
- the consumer's `ARCHITECTURE.md` (`persona-mind/`).

If your change adds a new request or reply variant, edit
`src/lib.rs` first, push, then update the consumer
(`persona-mind`) to handle it.

## What this repo owns

- `RoleName` (closed enum: Operator, OperatorAssistant,
  Designer, DesignerAssistant, SystemSpecialist,
  SystemAssistant, Poet, PoetAssistant, plus canonical
  workspace role token parsing/rendering).
- `ScopeReference` (closed enum: Path | Task) plus
  `WirePath` and `TaskToken` newtypes.
- `ScopeReason` (provisional `String` newtype).
- `TimestampNanos` (store-supplied; never agent-supplied).
- The typed mind graph substrate: `ThoughtKind` / `ThoughtBody`,
  `RelationKind`, `Thought`, `Relation`, `RecordId`, `RelationId`,
  thought/relation filters, subscription records, and graph
  commit/list replies.
- The closed `MindRequest` enum (`RoleClaim`,
  `RoleRelease`, `RoleHandoff`, `RoleObservation`,
  `ActivitySubmission`, `ActivityQuery`, `SubmitThought`,
  `SubmitRelation`, `QueryThoughts`, `QueryRelations`,
  `SubscribeThoughts`, `SubscribeRelations`, `Opening`,
  `NoteSubmission`, `Link`, `StatusChange`,
  `AliasAssignment`, `Query`).
- The closed `MindReply` enum (`ClaimAcceptance`,
  `ClaimRejection`, `ReleaseAcknowledgment`,
  `HandoffAcceptance`, `HandoffRejection`, `RoleSnapshot`,
  `ActivityAcknowledgment`, `ActivityList`, `ThoughtCommitted`,
  `RelationCommitted`, `ThoughtList`, `RelationList`,
  `SubscriptionAccepted`, `SubscriptionEvent`,
  `OpeningReceipt`, `NoteReceipt`, `LinkReceipt`,
  `StatusReceipt`, `AliasReceipt`, `View`, `Rejection`).
- The mind memory/work record vocabulary: `Item`, `Note`, `Edge`,
  `Event`, aliases, references, and ready-query records.
- The `Frame` type alias and round-trip tests.

## What this repo does not own

- The state actor or the database — that's
  `persona-mind`.
- The CLI binary parsing — that's the `orchestrate` bin
  compatibility shim and the `mind` bin target inside `persona-mind`.
- Lock-file projection writing — outside this implementation
  target; `persona-mind` replaces lock files instead of
  projecting them.
- The activity log retention policy — that's
  `persona-mind`.
- Storage tables — those live in `persona-mind`'s
  `src/tables.rs` (typed `sema::Table<K, V>` constants).
