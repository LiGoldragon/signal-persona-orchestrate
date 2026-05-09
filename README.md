# signal-persona-orchestrate

The Signal contract between **`orchestrate`** (the CLI agents
invoke per call) and **`persona-orchestrate`** (the typed
orchestration state actor that owns `orchestrate.redb`).

Read `src/lib.rs` for the public interface — two enums
(`OrchestrateRequest`, `OrchestrateReply`) declared via the
`signal_channel!` macro. The variants ARE the messages this
channel carries:

- **Role lifecycle:** `RoleClaim`, `RoleRelease`,
  `RoleHandoff`, `RoleObservation`.
- **Activity log:** `ActivitySubmission`, `ActivityQuery`.

## Quick reference

```rust
use signal_persona_orchestrate::{
    Frame, OrchestrateRequest, RoleClaim, RoleName, ScopeReason,
    ScopeReference, WirePath,
};
use signal_core::{FrameBody, Request};

// Designer claims a path and a task scope
let request = OrchestrateRequest::RoleClaim(RoleClaim {
    role: RoleName::Designer,
    scopes: vec![
        ScopeReference::Path(
            WirePath::from_absolute_path("/git/.../signal/ARCHITECTURE.md")?
        ),
    ],
    reason: ScopeReason::from_text("rescope per /91 §3.1")?,
});
let frame = Frame::new(FrameBody::Request(Request::assert(request)));
let bytes = frame.encode_length_prefixed()?;
// hand to persona-orchestrate's CLI dispatcher
```

The state actor replies with `OrchestrateReply::ClaimAcceptance`
on success or `OrchestrateReply::ClaimRejection` (carrying
typed `ScopeConflict` records) on overlap.

Use the public constructors for boundary strings before
building a frame: `WirePath::from_absolute_path` (which
stores a normalized absolute path),
`TaskToken::from_wire_token`, and `ScopeReason::from_text`.

## See also

- `ARCHITECTURE.md` — channel role + boundaries
- `~/primary/reports/designer/93-persona-orchestrate-rust-rewrite-and-activity-log.md`
  — the design report grounding this contract
- `~/primary/skills/contract-repo.md` — contract-repo
  discipline
- `signal-core` — kernel that supplies `Frame`,
  `Request`, `Reply`, `signal_channel!`
- `persona-orchestrate` — the consumer that implements
  this contract
