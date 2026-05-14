# signal-persona-mind

The Signal contract between **`mind`** (the CLI agents invoke per call)
and **`persona-mind`** (the central state actor that owns `mind.redb`).

Read `src/lib.rs` for the public interface — two enums
(`MindRequest`, `MindReply`) declared via the
`signal_channel!` macro. The variants ARE the messages this
channel carries:

- **Role lifecycle:** `RoleClaim`, `RoleRelease`,
  `RoleHandoff`, `RoleObservation`.
- **Activity log:** `ActivitySubmission`, `ActivityQuery`.
- **Memory/work graph:** `Opening`, `NoteSubmission`, `Link`,
  `StatusChange`, `AliasAssignment`, `Query`.

## Quick reference

```rust
use signal_persona_mind::{
    Frame, MindRequest, RoleClaim, RoleName, ScopeReason,
    ScopeReference, WirePath,
};
use signal_core::FrameBody;

// Designer claims a path and a task scope
let request = MindRequest::RoleClaim(RoleClaim {
    role: RoleName::Designer,
    scopes: vec![
        ScopeReference::Path(
            WirePath::from_absolute_path("/git/.../signal/ARCHITECTURE.md")?
        ),
    ],
    reason: ScopeReason::from_text("rescope per /91 §3.1")?,
});
let frame = Frame::new(FrameBody::Request(request.into_signal_request()));
let bytes = frame.encode_length_prefixed()?;
// hand to persona-mind's CLI dispatcher
```

The state actor replies with `MindReply::ClaimAcceptance`
on success or `MindReply::ClaimRejection` (carrying
typed `ScopeConflict` records) on overlap.

Use the public constructors for boundary strings before
building a frame: `WirePath::from_absolute_path` (which
stores a normalized absolute path),
`TaskToken::from_wire_token`, and `ScopeReason::from_text`.

## See also

- `ARCHITECTURE.md` — channel role + boundaries
- `~/primary/skills/contract-repo.md` — contract-repo
  discipline
- `signal-core` — kernel that supplies `Frame`,
  `Request`, `Reply`, `signal_channel!`
- `persona-mind` — the consumer that implements
  this contract
