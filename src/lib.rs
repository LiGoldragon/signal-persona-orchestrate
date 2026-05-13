//! Signal contract — `mind` CLI ↔ `persona-mind`.
//!
//! Read this file as the public interface of the central Persona
//! mind channel. The channel carries:
//!
//! - **Role claim/release/handoff** — the claim-flow today
//!   served by `tools/orchestrate` (a bash helper); migrating
//!   into `persona-mind`.
//! - **Role observation** — read the active claims for every
//!   role plus the most recent activity entries.
//! - **Activity submission** — append a typed activity record:
//!   who (role), what (path or task token), why (short reason).
//!   Time is store-stamped, never agent-supplied (per
//!   `~/primary/ESSENCE.md` §"Infrastructure mints identity,
//!   time, and sender").
//! - **Activity query** — read recent activity records,
//!   optionally filtered by role or scope.
//! - **Memory/work graph** — append typed item, note, edge,
//!   alias, and status events, then query the derived view.
//! - **Typed mind graph substrate** — submit/query/subscribe to
//!   closed Thought and Relation records (`Observation`, `Memory`,
//!   `Belief`, `Goal`, `Claim`, `Decision`, `Reference`) per
//!   designer/152.
//!
//! The channel is **request/reply** (every operation has a
//! typed reply). Subscription mode is a future extension —
//! see `~/primary/reports/operator/100-persona-mind-central-rename-plan.md`.
//!
//! See `ARCHITECTURE.md` for the channel's role and
//! boundaries; `~/primary/skills/contract-repo.md` for the
//! contract-repo discipline this crate follows.

use nota_codec::{
    Decoder, Encoder, NotaDecode, NotaEncode, NotaEnum, NotaRecord, NotaSum, NotaTransparent,
    NotaTryTransparent,
};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use signal_core::signal_channel;
use signal_persona_auth::{ChannelId, ComponentName, ConnectionClass, MessageOrigin};
use std::fmt;
use std::str::FromStr;

mod graph;
pub use graph::*;

// ─── Error ────────────────────────────────────────────────

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("wire path must be absolute and normalized: {path}")]
    InvalidWirePath { path: String },
    #[error("task token must be non-empty, unbracketed, and contain no whitespace: {token}")]
    InvalidTaskToken { token: String },
    #[error("scope reason must be non-empty and single-line: {reason}")]
    InvalidScopeReason { reason: String },
    #[error("unknown workspace role token: {role}")]
    UnknownRoleName { role: String },
}

// ─── Identity ─────────────────────────────────────────────

/// The closed set of workspace roles. Adding a role is a
/// coordinated schema change — every consumer of this
/// contract recompiles together.
#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    NotaEnum,
)]
pub enum RoleName {
    Operator,
    OperatorAssistant,
    Designer,
    DesignerAssistant,
    SystemSpecialist,
    SystemAssistant,
    Poet,
    PoetAssistant,
}

impl RoleName {
    pub const ALL: [Self; 8] = [
        Self::Operator,
        Self::OperatorAssistant,
        Self::Designer,
        Self::DesignerAssistant,
        Self::SystemSpecialist,
        Self::SystemAssistant,
        Self::Poet,
        Self::PoetAssistant,
    ];

    pub const fn as_wire_token(self) -> &'static str {
        match self {
            Self::Operator => "operator",
            Self::OperatorAssistant => "operator-assistant",
            Self::Designer => "designer",
            Self::DesignerAssistant => "designer-assistant",
            Self::SystemSpecialist => "system-specialist",
            Self::SystemAssistant => "system-assistant",
            Self::Poet => "poet",
            Self::PoetAssistant => "poet-assistant",
        }
    }

    pub fn from_wire_token(role: impl Into<String>) -> Result<Self> {
        let role = role.into();
        match role.as_str() {
            "operator" => Ok(Self::Operator),
            "operator-assistant" => Ok(Self::OperatorAssistant),
            "designer" => Ok(Self::Designer),
            "designer-assistant" => Ok(Self::DesignerAssistant),
            "system-specialist" => Ok(Self::SystemSpecialist),
            "system-assistant" => Ok(Self::SystemAssistant),
            "poet" => Ok(Self::Poet),
            "poet-assistant" => Ok(Self::PoetAssistant),
            _ => Err(Error::UnknownRoleName { role }),
        }
    }
}

impl fmt::Display for RoleName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_wire_token())
    }
}

impl FromStr for RoleName {
    type Err = Error;

    fn from_str(role: &str) -> Result<Self> {
        Self::from_wire_token(role)
    }
}

impl TryFrom<String> for RoleName {
    type Error = Error;

    fn try_from(role: String) -> Result<Self> {
        Self::from_wire_token(role)
    }
}

impl TryFrom<&str> for RoleName {
    type Error = Error;

    fn try_from(role: &str) -> Result<Self> {
        Self::from_wire_token(role)
    }
}

// ─── Scope reference ──────────────────────────────────────

/// What's being claimed / observed / acted on.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScopeReference {
    /// An absolute file or directory path.
    Path(WirePath),
    /// A bracketed task token like `[primary-f99]` (stored
    /// without brackets here).
    Task(TaskToken),
}

impl NotaEncode for ScopeReference {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Path(path) => {
                encoder.start_record("Path")?;
                path.encode(encoder)?;
                encoder.end_record()
            }
            Self::Task(task) => {
                encoder.start_record("Task")?;
                task.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ScopeReference {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Path" => {
                decoder.expect_record_head("Path")?;
                let path = WirePath::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Path(path))
            }
            "Task" => {
                decoder.expect_record_head("Task")?;
                let task = TaskToken::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Task(task))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ScopeReference",
                got: other.to_string(),
            }),
        }
    }
}

/// Absolute path, newtyped for cross-platform stability on
/// the wire (per `~/primary/skills/rust-discipline.md`
/// §"Newtype the wire form" — `PathBuf` archives
/// non-deterministically).
#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTryTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct WirePath(String);

impl WirePath {
    pub fn try_new(path: String) -> Result<Self> {
        Self::from_absolute_path(path)
    }

    pub fn from_absolute_path(path: impl Into<String>) -> Result<Self> {
        let path = path.into();

        if !path.starts_with('/') || path.split('/').any(|component| component == "..") {
            return Err(Error::InvalidWirePath { path });
        }

        let components = path
            .split('/')
            .filter(|component| !component.is_empty() && *component != ".")
            .collect::<Vec<_>>();
        let normalized = if components.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", components.join("/"))
        };

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WirePath {
    type Error = Error;

    fn try_from(path: String) -> Result<Self> {
        Self::from_absolute_path(path)
    }
}

impl TryFrom<&str> for WirePath {
    type Error = Error;

    fn try_from(path: &str) -> Result<Self> {
        Self::from_absolute_path(path)
    }
}

impl AsRef<str> for WirePath {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A bracketed task identifier (stored without brackets).
/// Bracketed form like `[primary-f99]` is the human surface;
/// the wire carries the raw token.
#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTryTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct TaskToken(String);

impl TaskToken {
    pub fn try_new(token: String) -> Result<Self> {
        Self::from_wire_token(token)
    }

    pub fn from_wire_token(token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        if token.is_empty()
            || token.contains('[')
            || token.contains(']')
            || token.chars().any(char::is_whitespace)
        {
            Err(Error::InvalidTaskToken { token })
        } else {
            Ok(Self(token))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TaskToken {
    type Error = Error;

    fn try_from(token: String) -> Result<Self> {
        Self::from_wire_token(token)
    }
}

impl TryFrom<&str> for TaskToken {
    type Error = Error;

    fn try_from(token: &str) -> Result<Self> {
        Self::from_wire_token(token)
    }
}

impl AsRef<str> for TaskToken {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ─── Reason ───────────────────────────────────────────────

/// A short reason string. Provisional per
/// `~/primary/reports/designer/92-sema-as-database-library-architecture-revamp.md`
/// §4 — strings allowed here until the typed Nexus record
/// shape for "intent" is named.
#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTryTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct ScopeReason(String);

impl ScopeReason {
    pub fn try_new(reason: String) -> Result<Self> {
        Self::from_text(reason)
    }

    pub fn from_text(reason: impl Into<String>) -> Result<Self> {
        let reason = reason.into();
        if reason.trim().is_empty() || reason.contains('\n') || reason.contains('\r') {
            Err(Error::InvalidScopeReason { reason })
        } else {
            Ok(Self(reason))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ScopeReason {
    type Error = Error;

    fn try_from(reason: String) -> Result<Self> {
        Self::from_text(reason)
    }
}

impl TryFrom<&str> for ScopeReason {
    type Error = Error;

    fn try_from(reason: &str) -> Result<Self> {
        Self::from_text(reason)
    }
}

impl AsRef<str> for ScopeReason {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

// ─── Time ─────────────────────────────────────────────────

/// Nanoseconds since the UNIX epoch. Store-supplied at
/// commit time; never agent-supplied.
#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    NotaTransparent,
)]
pub struct TimestampNanos(u64);

impl TimestampNanos {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

// ─── Claim verbs ──────────────────────────────────────────

/// A role asks to claim one or more scopes with a short
/// reason. Reply: `ClaimAcceptance` on success, `ClaimRejection`
/// listing every conflict on failure.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct RoleClaim {
    pub role: RoleName,
    pub scopes: Vec<ScopeReference>,
    pub reason: ScopeReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ClaimAcceptance {
    pub role: RoleName,
    pub scopes: Vec<ScopeReference>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ClaimRejection {
    pub role: RoleName,
    pub conflicts: Vec<ScopeConflict>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ScopeConflict {
    pub scope: ScopeReference,
    pub held_by: RoleName,
    pub held_reason: ScopeReason,
}

// ─── Release verbs ────────────────────────────────────────

/// A role releases all of its currently-held scopes.
/// Reply: `ReleaseAcknowledgment` listing what was released.
#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, Copy, PartialEq, Eq,
)]
pub struct RoleRelease {
    pub role: RoleName,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAcknowledgment {
    pub role: RoleName,
    pub released_scopes: Vec<ScopeReference>,
}

// ─── Handoff verbs ────────────────────────────────────────

/// One role hands a set of scopes to another role atomically.
/// Reply: `HandoffAcceptance` on success, `HandoffRejection`
/// with a typed reason on failure.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct RoleHandoff {
    pub from: RoleName,
    pub to: RoleName,
    pub scopes: Vec<ScopeReference>,
    pub reason: ScopeReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct HandoffAcceptance {
    pub from: RoleName,
    pub to: RoleName,
    pub scopes: Vec<ScopeReference>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct HandoffRejection {
    pub from: RoleName,
    pub to: RoleName,
    pub reason: HandoffRejectionReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum HandoffRejectionReason {
    /// The `from` role doesn't currently hold the named scopes.
    SourceRoleDoesNotHold,
    /// The `to` role's existing claims conflict with the
    /// scopes being handed off (the conflict list names which
    /// scopes and which existing holders).
    TargetRoleConflict(Vec<ScopeConflict>),
}

impl NotaEncode for HandoffRejectionReason {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::SourceRoleDoesNotHold => {
                encoder.start_record("SourceRoleDoesNotHold")?;
                encoder.end_record()
            }
            Self::TargetRoleConflict(conflicts) => {
                encoder.start_record("TargetRoleConflict")?;
                conflicts.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for HandoffRejectionReason {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "SourceRoleDoesNotHold" => {
                decoder.expect_record_head("SourceRoleDoesNotHold")?;
                decoder.expect_record_end()?;
                Ok(Self::SourceRoleDoesNotHold)
            }
            "TargetRoleConflict" => {
                decoder.expect_record_head("TargetRoleConflict")?;
                let conflicts = Vec::<ScopeConflict>::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::TargetRoleConflict(conflicts))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "HandoffRejectionReason",
                got: other.to_string(),
            }),
        }
    }
}

// ─── Observation ──────────────────────────────────────────

/// Request a snapshot of every role's active claims plus the
/// most recent activity entries. Reply: `RoleSnapshot`.
#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, Copy, PartialEq, Eq,
)]
pub struct RoleObservation;

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct RoleSnapshot {
    pub roles: Vec<RoleStatus>,
    pub recent_activity: Vec<Activity>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct RoleStatus {
    pub role: RoleName,
    pub claims: Vec<ClaimEntry>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ClaimEntry {
    pub scope: ScopeReference,
    pub reason: ScopeReason,
}

// ─── Activity log ─────────────────────────────────────────

/// One activity record: who touched what and why. Time is
/// store-supplied (per ESSENCE infrastructure-mints rule —
/// the agent never invents timestamps).
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub role: RoleName,
    pub scope: ScopeReference,
    pub reason: ScopeReason,
    pub stamped_at: TimestampNanos,
}

/// Submit a new activity record. The store assigns
/// `stamped_at` on commit. Reply: `ActivityAcknowledgment`
/// carrying the slot the record landed in.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ActivitySubmission {
    pub role: RoleName,
    pub scope: ScopeReference,
    pub reason: ScopeReason,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, Copy, PartialEq, Eq,
)]
pub struct ActivityAcknowledgment {
    /// The slot (sequential u64) the record was assigned.
    pub slot: u64,
}

/// Query the activity log. Limit caps how many records come
/// back; filters narrow by role or scope. Empty filter list
/// = "all".
#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ActivityQuery {
    pub limit: u32,
    pub filters: Vec<ActivityFilter>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum ActivityFilter {
    /// Only entries from this role.
    RoleFilter(RoleName),
    /// Only entries whose scope is `Path(p)` where `p`
    /// starts with this prefix.
    PathPrefix(WirePath),
    /// Only entries whose scope is the exact-match
    /// `Task(token)`.
    TaskToken(TaskToken),
}

impl NotaEncode for ActivityFilter {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::RoleFilter(role) => {
                encoder.start_record("RoleFilter")?;
                role.encode(encoder)?;
                encoder.end_record()
            }
            Self::PathPrefix(path) => {
                encoder.start_record("PathPrefix")?;
                path.encode(encoder)?;
                encoder.end_record()
            }
            Self::TaskToken(token) => {
                encoder.start_record("TaskToken")?;
                token.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ActivityFilter {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "RoleFilter" => {
                decoder.expect_record_head("RoleFilter")?;
                let role = RoleName::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::RoleFilter(role))
            }
            "PathPrefix" => {
                decoder.expect_record_head("PathPrefix")?;
                let path = WirePath::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::PathPrefix(path))
            }
            "TaskToken" => {
                decoder.expect_record_head("TaskToken")?;
                let token = TaskToken::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::TaskToken(token))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ActivityFilter",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ActivityList {
    /// Ordered most-recent first.
    pub records: Vec<Activity>,
}

// ─── Mind Memory Identity ─────────────────────────────────

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct StableItemId(String);

impl StableItemId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct DisplayId(String);

impl DisplayId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct ExternalAlias(String);

impl ExternalAlias {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct BeadsToken(String);

impl BeadsToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct OperationId(String);

impl OperationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct ActorName(String);

impl ActorName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    NotaTransparent,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub struct EventSeq(u64);

impl EventSeq {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn into_u64(self) -> u64 {
        self.0
    }
}

#[derive(
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
    NotaTransparent,
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub struct QueryLimit(u16);

impl QueryLimit {
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    pub fn into_u16(self) -> u16 {
        self.0
    }
}

// ─── Mind Memory Text ─────────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq)]
pub struct Title(String);

impl Title {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq)]
pub struct TextBody(String);

impl TextBody {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct ReportPath(String);

impl ReportPath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct ReferencePath(String);

impl ReferencePath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct CommitHash(String);

impl CommitHash {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ─── Mind Memory Domain ───────────────────────────────────

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum ItemKind {
    Task,
    Defect,
    Question,
    Decision,
    Note,
    Handoff,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum ItemStatus {
    Open,
    InProgress,
    Blocked,
    Closed,
    Deferred,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum ItemPriority {
    Critical,
    High,
    Normal,
    Low,
    Backlog,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum EdgeKind {
    DependsOn,
    ParentOf,
    RelatesTo,
    Duplicates,
    Supersedes,
    Answers,
    References,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemReference {
    Stable(StableItemId),
    Display(DisplayId),
    Alias(ExternalAlias),
}

impl NotaEncode for ItemReference {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Stable(id) => {
                encoder.start_record("Stable")?;
                id.encode(encoder)?;
                encoder.end_record()
            }
            Self::Display(id) => {
                encoder.start_record("Display")?;
                id.encode(encoder)?;
                encoder.end_record()
            }
            Self::Alias(alias) => {
                encoder.start_record("Alias")?;
                alias.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ItemReference {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Stable" => {
                decoder.expect_record_head("Stable")?;
                let id = StableItemId::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Stable(id))
            }
            "Display" => {
                decoder.expect_record_head("Display")?;
                let id = DisplayId::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Display(id))
            }
            "Alias" => {
                decoder.expect_record_head("Alias")?;
                let alias = ExternalAlias::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Alias(alias))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ItemReference",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExternalReference {
    Report(ReportPath),
    GitCommit(CommitHash),
    BeadsTask(BeadsToken),
    File(ReferencePath),
}

impl NotaEncode for ExternalReference {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Report(path) => {
                encoder.start_record("Report")?;
                path.encode(encoder)?;
                encoder.end_record()
            }
            Self::GitCommit(commit) => {
                encoder.start_record("GitCommit")?;
                commit.encode(encoder)?;
                encoder.end_record()
            }
            Self::BeadsTask(task) => {
                encoder.start_record("BeadsTask")?;
                task.encode(encoder)?;
                encoder.end_record()
            }
            Self::File(path) => {
                encoder.start_record("File")?;
                path.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ExternalReference {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Report" => {
                decoder.expect_record_head("Report")?;
                let path = ReportPath::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Report(path))
            }
            "GitCommit" => {
                decoder.expect_record_head("GitCommit")?;
                let commit = CommitHash::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::GitCommit(commit))
            }
            "BeadsTask" => {
                decoder.expect_record_head("BeadsTask")?;
                let task = BeadsToken::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::BeadsTask(task))
            }
            "File" => {
                decoder.expect_record_head("File")?;
                let path = ReferencePath::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::File(path))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ExternalReference",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LinkTarget {
    Item(ItemReference),
    External(ExternalReference),
}

impl NotaEncode for LinkTarget {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Item(item) => {
                encoder.start_record("Item")?;
                item.encode(encoder)?;
                encoder.end_record()
            }
            Self::External(external) => {
                encoder.start_record("External")?;
                external.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for LinkTarget {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Item" => {
                decoder.expect_record_head("Item")?;
                let item = ItemReference::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Item(item))
            }
            "External" => {
                decoder.expect_record_head("External")?;
                let external = ExternalReference::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::External(external))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "LinkTarget",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeTarget {
    Item(StableItemId),
    External(ExternalReference),
}

impl NotaEncode for EdgeTarget {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Item(item) => {
                encoder.start_record("Item")?;
                item.encode(encoder)?;
                encoder.end_record()
            }
            Self::External(external) => {
                encoder.start_record("External")?;
                external.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for EdgeTarget {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Item" => {
                decoder.expect_record_head("Item")?;
                let item = StableItemId::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Item(item))
            }
            "External" => {
                decoder.expect_record_head("External")?;
                let external = ExternalReference::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::External(external))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "EdgeTarget",
                got: other.to_string(),
            }),
        }
    }
}

// ─── Mind Memory Requests ─────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    pub kind: ItemKind,
    pub priority: ItemPriority,
    pub title: Title,
    pub body: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct NoteSubmission {
    pub item: ItemReference,
    pub body: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub source: ItemReference,
    pub kind: EdgeKind,
    pub target: LinkTarget,
    pub body: Option<TextBody>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct StatusChange {
    pub item: ItemReference,
    pub status: ItemStatus,
    pub body: Option<TextBody>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AliasAssignment {
    pub item: ItemReference,
    pub alias: ExternalAlias,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub kind: QueryKind,
    pub limit: QueryLimit,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum QueryKind {
    Ready,
    Blocked,
    Open,
    RecentEvents,
    ByItem(ItemReference),
    ByKind(ItemKind),
    ByStatus(ItemStatus),
    ByAlias(ExternalAlias),
}

impl NotaEncode for QueryKind {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Ready => {
                encoder.start_record("Ready")?;
                encoder.end_record()
            }
            Self::Blocked => {
                encoder.start_record("Blocked")?;
                encoder.end_record()
            }
            Self::Open => {
                encoder.start_record("Open")?;
                encoder.end_record()
            }
            Self::RecentEvents => {
                encoder.start_record("RecentEvents")?;
                encoder.end_record()
            }
            Self::ByItem(item) => {
                encoder.start_record("ByItem")?;
                item.encode(encoder)?;
                encoder.end_record()
            }
            Self::ByKind(kind) => {
                encoder.start_record("ByKind")?;
                kind.encode(encoder)?;
                encoder.end_record()
            }
            Self::ByStatus(status) => {
                encoder.start_record("ByStatus")?;
                status.encode(encoder)?;
                encoder.end_record()
            }
            Self::ByAlias(alias) => {
                encoder.start_record("ByAlias")?;
                alias.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for QueryKind {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Ready" => {
                decoder.expect_record_head("Ready")?;
                decoder.expect_record_end()?;
                Ok(Self::Ready)
            }
            "Blocked" => {
                decoder.expect_record_head("Blocked")?;
                decoder.expect_record_end()?;
                Ok(Self::Blocked)
            }
            "Open" => {
                decoder.expect_record_head("Open")?;
                decoder.expect_record_end()?;
                Ok(Self::Open)
            }
            "RecentEvents" => {
                decoder.expect_record_head("RecentEvents")?;
                decoder.expect_record_end()?;
                Ok(Self::RecentEvents)
            }
            "ByItem" => {
                decoder.expect_record_head("ByItem")?;
                let item = ItemReference::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::ByItem(item))
            }
            "ByKind" => {
                decoder.expect_record_head("ByKind")?;
                let kind = ItemKind::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::ByKind(kind))
            }
            "ByStatus" => {
                decoder.expect_record_head("ByStatus")?;
                let status = ItemStatus::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::ByStatus(status))
            }
            "ByAlias" => {
                decoder.expect_record_head("ByAlias")?;
                let alias = ExternalAlias::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::ByAlias(alias))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "QueryKind",
                got: other.to_string(),
            }),
        }
    }
}

// ─── Mind Memory Projections ──────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: StableItemId,
    pub display_id: DisplayId,
    pub aliases: Vec<ExternalAlias>,
    pub kind: ItemKind,
    pub status: ItemStatus,
    pub priority: ItemPriority,
    pub title: Title,
    pub body: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub event: EventSeq,
    pub item: StableItemId,
    pub author: ActorName,
    pub body: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub event: EventSeq,
    pub source: StableItemId,
    pub kind: EdgeKind,
    pub target: EdgeTarget,
    pub body: Option<TextBody>,
}

// ─── Mind Memory Events ───────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct EventHeader {
    pub event: EventSeq,
    pub operation: OperationId,
    pub actor: ActorName,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ItemOpenedEvent {
    pub header: EventHeader,
    pub item: Item,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct NoteAddedEvent {
    pub header: EventHeader,
    pub note: Note,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct EdgeAddedEvent {
    pub header: EventHeader,
    pub edge: Edge,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct StatusChangedEvent {
    pub header: EventHeader,
    pub item: StableItemId,
    pub status: ItemStatus,
    pub body: Option<TextBody>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AliasAddedEvent {
    pub header: EventHeader,
    pub item: StableItemId,
    pub alias: ExternalAlias,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaSum, Debug, Clone, PartialEq, Eq)]
pub enum Event {
    ItemOpened(ItemOpenedEvent),
    NoteAdded(NoteAddedEvent),
    EdgeAdded(EdgeAddedEvent),
    StatusChanged(StatusChangedEvent),
    AliasAdded(AliasAddedEvent),
}

// ─── Mind Memory Replies ──────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct OpeningReceipt {
    pub event: ItemOpenedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct NoteReceipt {
    pub event: NoteAddedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct LinkReceipt {
    pub event: EdgeAddedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct StatusReceipt {
    pub event: StatusChangedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AliasReceipt {
    pub event: AliasAddedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub items: Vec<Item>,
    pub edges: Vec<Edge>,
    pub notes: Vec<Note>,
    pub events: Vec<Event>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub reason: RejectionReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    UnknownItem,
    DuplicateAlias,
    InvalidEdge,
    PersistenceRejected,
    UnsupportedQuery,
    CollisionUnresolved,
}

// ─── Channel Choreography ─────────────────────────────────

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaTransparent, Debug, Clone, PartialEq, Eq, Hash,
)]
pub struct AdjudicationRequestId(String);

impl AdjudicationRequestId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum ChannelEndpoint {
    Internal(ComponentName),
    External(ConnectionClass),
}

impl NotaEncode for ChannelEndpoint {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Internal(component) => {
                encoder.start_record("Internal")?;
                component.encode(encoder)?;
                encoder.end_record()
            }
            Self::External(connection_class) => {
                encoder.start_record("External")?;
                connection_class.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ChannelEndpoint {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Internal" => {
                decoder.expect_record_head("Internal")?;
                let component = ComponentName::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Internal(component))
            }
            "External" => {
                decoder.expect_record_head("External")?;
                let connection_class = ConnectionClass::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::External(connection_class))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ChannelEndpoint",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum ChannelMessageKind {
    MessageIngressSubmission,
    MessageSubmission,
    InboxQuery,
    FocusObservation,
    PromptBufferObservation,
    MessageDelivery,
    TerminalInput,
    TerminalCapture,
    TerminalResize,
    TranscriptEvent,
    AdjudicationRequest,
    DeliveryNotification,
    ChannelGrant,
    ChannelExtend,
    ChannelRetract,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelDuration {
    OneShot,
    Permanent,
    TimeBound(TimestampNanos),
}

impl NotaEncode for ChannelDuration {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::OneShot => {
                encoder.start_record("OneShot")?;
                encoder.end_record()
            }
            Self::Permanent => {
                encoder.start_record("Permanent")?;
                encoder.end_record()
            }
            Self::TimeBound(timestamp) => {
                encoder.start_record("TimeBound")?;
                timestamp.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ChannelDuration {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "OneShot" => {
                decoder.expect_record_head("OneShot")?;
                decoder.expect_record_end()?;
                Ok(Self::OneShot)
            }
            "Permanent" => {
                decoder.expect_record_head("Permanent")?;
                decoder.expect_record_end()?;
                Ok(Self::Permanent)
            }
            "TimeBound" => {
                decoder.expect_record_head("TimeBound")?;
                let timestamp = TimestampNanos::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::TimeBound(timestamp))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ChannelDuration",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AdjudicationRequest {
    pub request: AdjudicationRequestId,
    pub origin: MessageOrigin,
    pub destination: ChannelEndpoint,
    pub kind: ChannelMessageKind,
    pub body_summary: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AdjudicationReceipt {
    pub request: AdjudicationRequestId,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelGrant {
    pub source: ChannelEndpoint,
    pub destination: ChannelEndpoint,
    pub kinds: Vec<ChannelMessageKind>,
    pub duration: ChannelDuration,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelExtend {
    pub channel: ChannelId,
    pub duration: ChannelDuration,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelRetract {
    pub channel: ChannelId,
    pub reason: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AdjudicationDeny {
    pub request: AdjudicationRequestId,
    pub reason: TextBody,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelList {
    pub filters: Vec<ChannelFilter>,
}

#[derive(
    Archive, RkyvSerialize, RkyvDeserialize, NotaEnum, Debug, Clone, Copy, PartialEq, Eq, Hash,
)]
pub enum MindOperationKind {
    SubmitThought,
    SubmitRelation,
    QueryThoughts,
    QueryRelations,
    SubscribeThoughts,
    SubscribeRelations,
    RoleClaim,
    RoleRelease,
    RoleHandoff,
    RoleObservation,
    ActivitySubmission,
    ActivityQuery,
    Opening,
    NoteSubmission,
    Link,
    StatusChange,
    AliasAssignment,
    Query,
    AdjudicationRequest,
    ChannelGrant,
    ChannelExtend,
    ChannelRetract,
    AdjudicationDeny,
    ChannelList,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum ChannelFilter {
    Source(ChannelEndpoint),
    Destination(ChannelEndpoint),
    Kind(ChannelMessageKind),
}

impl NotaEncode for ChannelFilter {
    fn encode(&self, encoder: &mut Encoder) -> nota_codec::Result<()> {
        match self {
            Self::Source(endpoint) => {
                encoder.start_record("Source")?;
                endpoint.encode(encoder)?;
                encoder.end_record()
            }
            Self::Destination(endpoint) => {
                encoder.start_record("Destination")?;
                endpoint.encode(encoder)?;
                encoder.end_record()
            }
            Self::Kind(kind) => {
                encoder.start_record("Kind")?;
                kind.encode(encoder)?;
                encoder.end_record()
            }
        }
    }
}

impl NotaDecode for ChannelFilter {
    fn decode(decoder: &mut Decoder<'_>) -> nota_codec::Result<Self> {
        let head = decoder.peek_record_head()?;
        match head.as_str() {
            "Source" => {
                decoder.expect_record_head("Source")?;
                let endpoint = ChannelEndpoint::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Source(endpoint))
            }
            "Destination" => {
                decoder.expect_record_head("Destination")?;
                let endpoint = ChannelEndpoint::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Destination(endpoint))
            }
            "Kind" => {
                decoder.expect_record_head("Kind")?;
                let kind = ChannelMessageKind::decode(decoder)?;
                decoder.expect_record_end()?;
                Ok(Self::Kind(kind))
            }
            other => Err(nota_codec::Error::UnknownKindForVerb {
                verb: "ChannelFilter",
                got: other.to_string(),
            }),
        }
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelReceipt {
    pub channel: ChannelId,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct AdjudicationDenyReceipt {
    pub request: AdjudicationRequestId,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelView {
    pub channel: ChannelId,
    pub source: ChannelEndpoint,
    pub destination: ChannelEndpoint,
    pub kinds: Vec<ChannelMessageKind>,
    pub duration: ChannelDuration,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, NotaRecord, Debug, Clone, PartialEq, Eq)]
pub struct ChannelListView {
    pub channels: Vec<ChannelView>,
}

// ─── Channel declaration ──────────────────────────────────

signal_channel! {
    request MindRequest {
        SubmitThought(SubmitThought),
        SubmitRelation(SubmitRelation),
        QueryThoughts(QueryThoughts),
        QueryRelations(QueryRelations),
        SubscribeThoughts(SubscribeThoughts),
        SubscribeRelations(SubscribeRelations),
        RoleClaim(RoleClaim),
        RoleRelease(RoleRelease),
        RoleHandoff(RoleHandoff),
        RoleObservation(RoleObservation),
        ActivitySubmission(ActivitySubmission),
        ActivityQuery(ActivityQuery),
        Opening(Opening),
        NoteSubmission(NoteSubmission),
        Link(Link),
        StatusChange(StatusChange),
        AliasAssignment(AliasAssignment),
        Query(Query),
        AdjudicationRequest(AdjudicationRequest),
        ChannelGrant(ChannelGrant),
        ChannelExtend(ChannelExtend),
        ChannelRetract(ChannelRetract),
        AdjudicationDeny(AdjudicationDeny),
        ChannelList(ChannelList),
    }
    reply MindReply {
        ThoughtCommitted(ThoughtCommitted),
        RelationCommitted(RelationCommitted),
        ThoughtList(ThoughtList),
        RelationList(RelationList),
        SubscriptionAccepted(SubscriptionAccepted),
        SubscriptionEvent(SubscriptionEvent),
        ClaimAcceptance(ClaimAcceptance),
        ClaimRejection(ClaimRejection),
        ReleaseAcknowledgment(ReleaseAcknowledgment),
        HandoffAcceptance(HandoffAcceptance),
        HandoffRejection(HandoffRejection),
        RoleSnapshot(RoleSnapshot),
        ActivityAcknowledgment(ActivityAcknowledgment),
        ActivityList(ActivityList),
        OpeningReceipt(OpeningReceipt),
        NoteReceipt(NoteReceipt),
        LinkReceipt(LinkReceipt),
        StatusReceipt(StatusReceipt),
        AliasReceipt(AliasReceipt),
        View(View),
        Rejection(Rejection),
        AdjudicationReceipt(AdjudicationReceipt),
        ChannelReceipt(ChannelReceipt),
        AdjudicationDenyReceipt(AdjudicationDenyReceipt),
        ChannelListView(ChannelListView),
        MindRequestUnimplemented(MindRequestUnimplemented),
    }
}

impl MindRequest {
    pub fn operation_kind(&self) -> MindOperationKind {
        match self {
            Self::SubmitThought(_) => MindOperationKind::SubmitThought,
            Self::SubmitRelation(_) => MindOperationKind::SubmitRelation,
            Self::QueryThoughts(_) => MindOperationKind::QueryThoughts,
            Self::QueryRelations(_) => MindOperationKind::QueryRelations,
            Self::SubscribeThoughts(_) => MindOperationKind::SubscribeThoughts,
            Self::SubscribeRelations(_) => MindOperationKind::SubscribeRelations,
            Self::RoleClaim(_) => MindOperationKind::RoleClaim,
            Self::RoleRelease(_) => MindOperationKind::RoleRelease,
            Self::RoleHandoff(_) => MindOperationKind::RoleHandoff,
            Self::RoleObservation(_) => MindOperationKind::RoleObservation,
            Self::ActivitySubmission(_) => MindOperationKind::ActivitySubmission,
            Self::ActivityQuery(_) => MindOperationKind::ActivityQuery,
            Self::Opening(_) => MindOperationKind::Opening,
            Self::NoteSubmission(_) => MindOperationKind::NoteSubmission,
            Self::Link(_) => MindOperationKind::Link,
            Self::StatusChange(_) => MindOperationKind::StatusChange,
            Self::AliasAssignment(_) => MindOperationKind::AliasAssignment,
            Self::Query(_) => MindOperationKind::Query,
            Self::AdjudicationRequest(_) => MindOperationKind::AdjudicationRequest,
            Self::ChannelGrant(_) => MindOperationKind::ChannelGrant,
            Self::ChannelExtend(_) => MindOperationKind::ChannelExtend,
            Self::ChannelRetract(_) => MindOperationKind::ChannelRetract,
            Self::AdjudicationDeny(_) => MindOperationKind::AdjudicationDeny,
            Self::ChannelList(_) => MindOperationKind::ChannelList,
        }
    }
}
