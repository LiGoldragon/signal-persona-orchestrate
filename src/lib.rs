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
//!
//! The channel is **request/reply** (every operation has a
//! typed reply). Subscription mode is a future extension —
//! see `~/primary/reports/operator/100-persona-mind-central-rename-plan.md`.
//!
//! See `ARCHITECTURE.md` for the channel's role and
//! boundaries; `~/primary/skills/contract-repo.md` for the
//! contract-repo discipline this crate follows.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use signal_core::signal_channel;

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
)]
pub enum RoleName {
    Operator,
    Designer,
    SystemSpecialist,
    Poet,
    Assistant,
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

/// Absolute path, newtyped for cross-platform stability on
/// the wire (per `~/primary/skills/rust-discipline.md`
/// §"Newtype the wire form" — `PathBuf` archives
/// non-deterministically).
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WirePath(String);

impl WirePath {
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
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskToken(String);

impl TaskToken {
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
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScopeReason(String);

impl ScopeReason {
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
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoleClaim {
    pub role: RoleName,
    pub scopes: Vec<ScopeReference>,
    pub reason: ScopeReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ClaimAcceptance {
    pub role: RoleName,
    pub scopes: Vec<ScopeReference>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ClaimRejection {
    pub role: RoleName,
    pub conflicts: Vec<ScopeConflict>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ScopeConflict {
    pub scope: ScopeReference,
    pub held_by: RoleName,
    pub held_reason: ScopeReason,
}

// ─── Release verbs ────────────────────────────────────────

/// A role releases all of its currently-held scopes.
/// Reply: `ReleaseAcknowledgment` listing what was released.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleRelease {
    pub role: RoleName,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ReleaseAcknowledgment {
    pub role: RoleName,
    pub released_scopes: Vec<ScopeReference>,
}

// ─── Handoff verbs ────────────────────────────────────────

/// One role hands a set of scopes to another role atomically.
/// Reply: `HandoffAcceptance` on success, `HandoffRejection`
/// with a typed reason on failure.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoleHandoff {
    pub from: RoleName,
    pub to: RoleName,
    pub scopes: Vec<ScopeReference>,
    pub reason: ScopeReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct HandoffAcceptance {
    pub from: RoleName,
    pub to: RoleName,
    pub scopes: Vec<ScopeReference>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
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

// ─── Observation ──────────────────────────────────────────

/// Request a snapshot of every role's active claims plus the
/// most recent activity entries. Reply: `RoleSnapshot`.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleObservation;

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoleSnapshot {
    pub roles: Vec<RoleStatus>,
    pub recent_activity: Vec<Activity>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoleStatus {
    pub role: RoleName,
    pub claims: Vec<ClaimEntry>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ClaimEntry {
    pub scope: ScopeReference,
    pub reason: ScopeReason,
}

// ─── Activity log ─────────────────────────────────────────

/// One activity record: who touched what and why. Time is
/// store-supplied (per ESSENCE infrastructure-mints rule —
/// the agent never invents timestamps).
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Activity {
    pub role: RoleName,
    pub scope: ScopeReference,
    pub reason: ScopeReason,
    pub stamped_at: TimestampNanos,
}

/// Submit a new activity record. The store assigns
/// `stamped_at` on commit. Reply: `ActivityAcknowledgment`
/// carrying the slot the record landed in.
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ActivitySubmission {
    pub role: RoleName,
    pub scope: ScopeReference,
    pub reason: ScopeReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityAcknowledgment {
    /// The slot (sequential u64) the record was assigned.
    pub slot: u64,
}

/// Query the activity log. Limit caps how many records come
/// back; filters narrow by role or scope. Empty filter list
/// = "all".
#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
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

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ActivityList {
    /// Ordered most-recent first.
    pub records: Vec<Activity>,
}

// ─── Mind Memory Identity ─────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StableItemId(String);

impl StableItemId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DisplayId(String);

impl DisplayId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalAlias(String);

impl ExternalAlias {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BeadsToken(String);

impl BeadsToken {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OperationId(String);

impl OperationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActorName(String);

impl ActorName {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventSeq(u64);

impl EventSeq {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn into_u64(self) -> u64 {
        self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Title(String);

impl Title {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Body(String);

impl Body {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReportPath(String);

impl ReportPath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferencePath(String);

impl ReferencePath {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Task,
    Defect,
    Question,
    Decision,
    Note,
    Handoff,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Open,
    InProgress,
    Blocked,
    Closed,
    Deferred,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Priority {
    Critical,
    High,
    Normal,
    Low,
    Backlog,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExternalReference {
    Report(ReportPath),
    GitCommit(CommitHash),
    BeadsTask(BeadsToken),
    File(ReferencePath),
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum LinkTarget {
    Item(ItemReference),
    External(ExternalReference),
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeTarget {
    Item(StableItemId),
    External(ExternalReference),
}

// ─── Mind Memory Requests ─────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    pub kind: Kind,
    pub priority: Priority,
    pub title: Title,
    pub body: Body,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct NoteSubmission {
    pub item: ItemReference,
    pub body: Body,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub source: ItemReference,
    pub kind: EdgeKind,
    pub target: LinkTarget,
    pub body: Option<Body>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct StatusChange {
    pub item: ItemReference,
    pub status: Status,
    pub body: Option<Body>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct AliasAssignment {
    pub item: ItemReference,
    pub alias: ExternalAlias,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
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
    ByKind(Kind),
    ByStatus(Status),
    ByAlias(ExternalAlias),
}

// ─── Mind Memory Projections ──────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: StableItemId,
    pub display_id: DisplayId,
    pub aliases: Vec<ExternalAlias>,
    pub kind: Kind,
    pub status: Status,
    pub priority: Priority,
    pub title: Title,
    pub body: Body,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub event: EventSeq,
    pub item: StableItemId,
    pub author: ActorName,
    pub body: Body,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub event: EventSeq,
    pub source: StableItemId,
    pub kind: EdgeKind,
    pub target: EdgeTarget,
    pub body: Option<Body>,
}

// ─── Mind Memory Events ───────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventHeader {
    pub event: EventSeq,
    pub operation: OperationId,
    pub actor: ActorName,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ItemOpenedEvent {
    pub header: EventHeader,
    pub item: Item,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct NoteAddedEvent {
    pub header: EventHeader,
    pub note: Note,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct EdgeAddedEvent {
    pub header: EventHeader,
    pub edge: Edge,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct StatusChangedEvent {
    pub header: EventHeader,
    pub item: StableItemId,
    pub status: Status,
    pub body: Option<Body>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct AliasAddedEvent {
    pub header: EventHeader,
    pub item: StableItemId,
    pub alias: ExternalAlias,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum Event {
    ItemOpened(ItemOpenedEvent),
    NoteAdded(NoteAddedEvent),
    EdgeAdded(EdgeAddedEvent),
    StatusChanged(StatusChangedEvent),
    AliasAdded(AliasAddedEvent),
}

// ─── Mind Memory Replies ──────────────────────────────────

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct OpeningReceipt {
    pub event: ItemOpenedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct NoteReceipt {
    pub event: NoteAddedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct LinkReceipt {
    pub event: EdgeAddedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct StatusReceipt {
    pub event: StatusChangedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct AliasReceipt {
    pub event: AliasAddedEvent,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub items: Vec<Item>,
    pub edges: Vec<Edge>,
    pub notes: Vec<Note>,
    pub events: Vec<Event>,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    pub reason: RejectionReason,
}

#[derive(Archive, RkyvSerialize, RkyvDeserialize, Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    UnknownItem,
    DuplicateAlias,
    InvalidEdge,
    PersistenceRejected,
    UnsupportedQuery,
    CollisionUnresolved,
}

// ─── Channel declaration ──────────────────────────────────

signal_channel! {
    request MindRequest {
        RoleClaim(RoleClaim),
        RoleRelease(RoleRelease),
        RoleHandoff(RoleHandoff),
        RoleObservation(RoleObservation),
        ActivitySubmission(ActivitySubmission),
        ActivityQuery(ActivityQuery),
        Open(Opening),
        AddNote(NoteSubmission),
        Link(Link),
        ChangeStatus(StatusChange),
        AddAlias(AliasAssignment),
        Query(Query),
    }
    reply MindReply {
        ClaimAcceptance(ClaimAcceptance),
        ClaimRejection(ClaimRejection),
        ReleaseAcknowledgment(ReleaseAcknowledgment),
        HandoffAcceptance(HandoffAcceptance),
        HandoffRejection(HandoffRejection),
        RoleSnapshot(RoleSnapshot),
        ActivityAcknowledgment(ActivityAcknowledgment),
        ActivityList(ActivityList),
        Opened(OpeningReceipt),
        NoteAdded(NoteReceipt),
        Linked(LinkReceipt),
        StatusChanged(StatusReceipt),
        AliasAdded(AliasReceipt),
        View(View),
        Rejected(Rejection),
    }
}
