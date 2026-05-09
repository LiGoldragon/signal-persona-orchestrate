# ARCHITECTURE — signal-persona-orchestrate

The Signal contract between **`orchestrate`** (the CLI client
agents invoke per call) and **`persona-orchestrate`** (the
typed orchestration state actor that owns
`orchestrate.redb`). The whole channel is one
`signal_channel!` invocation in `src/lib.rs`.

## Channel

| Side | Component |
|---|---|
| Request side | `orchestrate` CLI (one Nota record per agent invocation; constructed inside the binary's argv parser) |
| Reply side | `persona-orchestrate` state actor |

The CLI opens the database, runs one operation, prints one
reply, exits. Concurrent invocations serialize at the redb
level; multiple readers run in parallel.

## Record source

This contract defines its records locally (`RoleName`,
`ScopeReference`, `WirePath`, `TaskToken`, `ScopeReason`,
`TimestampNanos`, etc.) because they're the channel's
vocabulary, not records that travel beyond.

If a future channel needs `RoleName` (a workspace-coordination
broadcast channel, for instance), lift it to a shared
contract or to `signal-persona`'s umbrella records. For now,
local.

## Messages

```
OrchestrateRequest                 OrchestrateReply
├─ RoleClaim                       ├─ ClaimAcceptance
├─ RoleRelease                     ├─ ClaimRejection
├─ RoleHandoff                     ├─ ReleaseAcknowledgment
├─ RoleObservation                 ├─ HandoffAcceptance
├─ ActivitySubmission              ├─ HandoffRejection
└─ ActivityQuery                   ├─ RoleSnapshot
                                   ├─ ActivityAcknowledgment
                                   └─ ActivityList
```

Closed enums; no `Unknown` variant. Conflicts and rejections
carry typed reasons (`ScopeConflict`, `HandoffRejectionReason`)
so callers pattern-match on them rather than parsing strings.

## Versioning

`signal_core::Frame` carries the protocol version. Schema-level
changes (adding/removing variants, adding fields) are breaking
and require a coordinated upgrade of both `orchestrate` clients
and `persona-orchestrate`.

## Examples

```text
;; agent claims paths + task scope
OrchestrateRequest::RoleClaim(RoleClaim {
    role: RoleName::Designer,
    scopes: vec![
        ScopeReference::Path(WirePath::new("/git/.../signal/ARCHITECTURE.md")),
        ScopeReference::Task(TaskToken::new("primary-f99")),
    ],
    reason: ScopeReason::new("rescope per /91 §3.1"),
})

;; on success
OrchestrateReply::ClaimAcceptance(ClaimAcceptance {
    role: RoleName::Designer,
    scopes: vec![/* echoed */],
})

;; on conflict
OrchestrateReply::ClaimRejection(ClaimRejection {
    role: RoleName::Designer,
    conflicts: vec![ScopeConflict {
        scope: ScopeReference::Path(WirePath::new("/git/.../signal/ARCHITECTURE.md")),
        held_by: RoleName::Operator,
        held_reason: ScopeReason::new("Persona-prefix sweep"),
    }],
})

;; agent files an activity entry
OrchestrateRequest::ActivitySubmission(ActivitySubmission {
    role: RoleName::Operator,
    scope: ScopeReference::Path(WirePath::new("/git/.../persona-router/src/router.rs")),
    reason: ScopeReason::new("RouterActor consumes signal-persona-system Frame"),
})

;; reply
OrchestrateReply::ActivityAcknowledgment(ActivityAcknowledgment { slot: 1024 })
```

## Round trips

Per-variant round-trip tests in `tests/round_trip.rs` covering
all 6 request variants + all 8 reply variants + both
ScopeReference variants + handoff rejection sub-variants.

Architectural-truth tests fire when:

- A new variant is added without a round-trip test.
- The Frame's encode/decode bytes don't match.
- A consumer tries to dispatch on a variant that isn't in
  the closed enum.

## Non-ownership

- No state actor — that's `persona-orchestrate`.
- No CLI binary — that's `persona-orchestrate`'s
  `orchestrate` bin target.
- No database — the typed records persist in
  `persona-orchestrate`'s `orchestrate.redb`, opened
  through `persona-sema` (which uses the workspace's `sema`
  database library underneath).
- No transport (UDS path, reconnect, timeouts) — per
  consumer, though for v1 the CLI invokes the state actor
  in-process (no transport layer at all).
- Time supply — `Activity::stamped_at` is store-supplied;
  the `ActivitySubmission` request does not carry it (per
  ESSENCE infrastructure-mints rule).
- Lock-file projections — the state actor writes
  `<role>.lock` files for backward compatibility; this
  contract doesn't describe the projection format.

## Code map

```
src/
└── lib.rs    — payloads + signal_channel! invocation
tests/
└── round_trip.rs — per-variant wire-form round trips
```

## See also

- `~/primary/reports/designer/93-persona-orchestrate-rust-rewrite-and-activity-log.md`
  — the design report grounding this contract.
- `~/primary/reports/designer/4-persona-messaging-design.md`
  — the original Persona messaging design naming the
  orchestration ops.
- `~/primary/reports/designer/81-three-agent-orchestration-with-assistant-role.md`
  — the orchestration-pair model.
- `~/primary/skills/contract-repo.md` — contract-repo
  discipline this crate follows.
- `~/primary/skills/architectural-truth-tests.md` — the
  witness-test pattern.
- `~/primary/protocols/orchestration.md` — the current
  protocol; updated post-Rust-impl.
- `signal-core/src/channel.rs` — the `signal_channel!`
  macro this contract uses.
- `signal-persona-system/ARCHITECTURE.md` — peer channel;
  same shape.
