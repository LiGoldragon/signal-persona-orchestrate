# ARCHITECTURE — signal-persona-mind

The Signal contract between **`mind`** (the CLI client
agents invoke per call) and **`persona-mind`** (the
central state actor that owns `mind.redb`). The whole channel is one
`signal_channel!` invocation in `src/lib.rs`.

## Channel

| Side | Component |
|---|---|
| Request side | `mind` CLI (one Nota record per agent invocation; constructed inside the binary's argv parser) |
| Reply side | `persona-mind` state actor |

The CLI opens the database, runs one operation, prints one
reply, exits. Concurrent invocations serialize at the redb
level; multiple readers run in parallel.

## Record source

This contract defines its records locally (`RoleName`,
`ScopeReference`, `WirePath`, `TaskToken`, `ScopeReason`,
`TimestampNanos`, item identifiers, notes, edges, events, etc.) because
they're the mind channel's vocabulary, not records that travel beyond.

Boundary strings are validated at construction time:
`WirePath` accepts absolute paths and stores a normalized
slash-separated form, `TaskToken` accepts raw unbracketed
tokens with no whitespace, and `ScopeReason` accepts non-empty
single-line text. The parser/consumer boundary should use
these constructors before building a frame.

If a future channel needs `RoleName` (a workspace-coordination
broadcast channel, for instance), lift it to a shared
contract or to `signal-persona`'s umbrella records. For now,
local.

## Messages

```
MindRequest                 MindReply
├─ RoleClaim                       ├─ ClaimAcceptance
├─ RoleRelease                     ├─ ClaimRejection
├─ RoleHandoff                     ├─ ReleaseAcknowledgment
├─ RoleObservation                 ├─ HandoffAcceptance
├─ ActivitySubmission              ├─ HandoffRejection
├─ ActivityQuery                   ├─ RoleSnapshot
├─ Open                            ├─ ActivityAcknowledgment
├─ AddNote                         ├─ ActivityList
├─ Link                            ├─ Opened
├─ ChangeStatus                    ├─ NoteAdded
├─ AddAlias                        ├─ Linked
└─ Query                           ├─ StatusChanged
                                   ├─ AliasAdded
                                   ├─ View
                                   └─ Rejected
```

Closed enums; no `Unknown` variant. Conflicts and rejections
carry typed reasons (`ScopeConflict`, `HandoffRejectionReason`)
so callers pattern-match on them rather than parsing strings.

## Versioning

`signal_core::Frame` carries the protocol version. Schema-level
changes (adding/removing variants, adding fields) are breaking
and require a coordinated upgrade of both `mind` clients
and `persona-mind`.

## Examples

```text
;; agent claims paths + task scope
MindRequest::RoleClaim(RoleClaim {
    role: RoleName::Designer,
    scopes: vec![
        ScopeReference::Path(WirePath::from_absolute_path("/git/.../signal/ARCHITECTURE.md")?),
        ScopeReference::Task(TaskToken::from_wire_token("primary-f99")?),
    ],
    reason: ScopeReason::from_text("rescope per /91 §3.1")?,
})

;; on success
MindReply::ClaimAcceptance(ClaimAcceptance {
    role: RoleName::Designer,
    scopes: vec![/* echoed */],
})

;; on conflict
MindReply::ClaimRejection(ClaimRejection {
    role: RoleName::Designer,
    conflicts: vec![ScopeConflict {
        scope: ScopeReference::Path(WirePath::from_absolute_path("/git/.../signal/ARCHITECTURE.md")?),
        held_by: RoleName::Operator,
        held_reason: ScopeReason::from_text("Persona-prefix sweep")?,
    }],
})

;; agent files an activity entry
MindRequest::ActivitySubmission(ActivitySubmission {
    role: RoleName::Operator,
    scope: ScopeReference::Path(WirePath::from_absolute_path("/git/.../persona-router/src/router.rs")?),
    reason: ScopeReason::from_text("RouterActor consumes signal-persona-system Frame")?,
})

;; reply
MindReply::ActivityAcknowledgment(ActivityAcknowledgment { slot: 1024 })

;; open a memory/work item
MindRequest::Open(Opening {
    kind: Kind::Task,
    priority: Priority::High,
    title: Title::new("Replace BEADS"),
    body: Body::new("Open a typed mind item."),
})
```

## Round trips

Per-variant round-trip tests in `tests/round_trip.rs` covering
all role/activity request variants, all memory/work request variants,
all reply variants, both `ScopeReference` variants, every `EdgeKind`,
every `QueryKind`, and every external-reference target.

Architectural-truth tests fire when:

- A new variant is added without a round-trip test.
- The Frame's encode/decode bytes don't match.
- A consumer tries to dispatch on a variant that isn't in
  the closed enum.

## Non-ownership

- No state actor — that's `persona-mind`.
- No CLI binary — that's `persona-mind`'s
  `mind` bin target.
- No database — the typed records persist in
  `persona-mind`'s `mind.redb`, opened
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

- `~/primary/reports/operator/100-persona-mind-central-rename-plan.md`
  — the current design report grounding this contract.
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
