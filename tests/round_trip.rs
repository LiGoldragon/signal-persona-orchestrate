//! Architectural-truth round-trip tests for the
//! `signal-persona-mind` channel.
//!
//! Per `~/primary/skills/architectural-truth-tests.md`,
//! each variant of both enums has a witness test that
//! proves the macro-emitted type round-trips through a
//! length-prefixed Frame.

use signal_core::{FrameBody, Reply, Request, SemaVerb};
use signal_persona_mind::{
    Activity, ActivityAcknowledgment, ActivityFilter, ActivityList, ActivityQuery,
    ActivitySubmission, ActorName, AliasAddedEvent, AliasAssignment, AliasReceipt, BeadsToken,
    ClaimAcceptance, ClaimEntry, ClaimRejection, CommitHash, DisplayId, Edge, EdgeAddedEvent,
    EdgeKind, EdgeTarget, Event, EventHeader, EventSeq, ExternalAlias, ExternalReference, Frame,
    HandoffAcceptance, HandoffRejection, HandoffRejectionReason, Item, ItemKind, ItemOpenedEvent,
    ItemPriority, ItemReference, ItemStatus, Link, LinkReceipt, LinkTarget, MindReply, MindRequest,
    Note, NoteAddedEvent, NoteReceipt, NoteSubmission, Opening, OpeningReceipt, OperationId, Query,
    QueryKind, QueryLimit, ReferencePath, Rejection, RejectionReason, ReleaseAcknowledgment,
    ReportPath, RoleClaim, RoleHandoff, RoleName, RoleObservation, RoleRelease, RoleSnapshot,
    RoleStatus, ScopeConflict, ScopeReason, ScopeReference, StableItemId, StatusChange,
    StatusChangedEvent, StatusReceipt, TaskToken, TextBody, TimestampNanos, Title, View, WirePath,
};

// ─── Helpers ──────────────────────────────────────────────

fn round_trip_request(request: MindRequest) -> MindRequest {
    let frame = Frame::new(FrameBody::Request(Request::assert(request)));
    let bytes = frame.encode_length_prefixed().expect("encode");
    let decoded = Frame::decode_length_prefixed(&bytes).expect("decode");
    match decoded.into_body() {
        FrameBody::Request(Request::Operation { verb, payload }) => {
            assert_eq!(verb, SemaVerb::Assert);
            payload
        }
        other => panic!("expected request operation, got {other:?}"),
    }
}

fn round_trip_reply(reply: MindReply) -> MindReply {
    let frame = Frame::new(FrameBody::Reply(Reply::operation(reply)));
    let bytes = frame.encode_length_prefixed().expect("encode");
    let decoded = Frame::decode_length_prefixed(&bytes).expect("decode");
    match decoded.into_body() {
        FrameBody::Reply(Reply::Operation(reply)) => reply,
        other => panic!("expected reply operation, got {other:?}"),
    }
}

fn sample_path() -> WirePath {
    WirePath::from_absolute_path("/git/github.com/LiGoldragon/signal-persona-mind/src/lib.rs")
        .expect("absolute path")
}

fn sample_task() -> TaskToken {
    TaskToken::from_wire_token("primary-f99").expect("wire task token")
}

fn sample_reason() -> ScopeReason {
    ScopeReason::from_text("design-cascade per /93").expect("scope reason")
}

fn sample_path_scope() -> ScopeReference {
    ScopeReference::Path(sample_path())
}

fn sample_task_scope() -> ScopeReference {
    ScopeReference::Task(sample_task())
}

struct MemoryFixture {
    item_id: StableItemId,
    display_id: DisplayId,
    actor: ActorName,
    operation: OperationId,
}

impl MemoryFixture {
    fn new() -> Self {
        Self {
            item_id: StableItemId::new("item-0000000000000001"),
            display_id: DisplayId::new("9iv"),
            actor: ActorName::new("operator"),
            operation: OperationId::new("op-0000000000000001"),
        }
    }

    fn header(&self, event: u64) -> EventHeader {
        EventHeader {
            event: EventSeq::new(event),
            operation: self.operation.clone(),
            actor: self.actor.clone(),
        }
    }

    fn item(&self) -> Item {
        Item {
            id: self.item_id.clone(),
            display_id: self.display_id.clone(),
            aliases: vec![ExternalAlias::new("primary-9iv")],
            kind: ItemKind::Task,
            status: ItemStatus::Open,
            priority: ItemPriority::High,
            title: Title::new("Implement native mind memory graph"),
            body: TextBody::new("Replace BEADS with typed Persona mind records."),
        }
    }

    fn opened_event(&self) -> ItemOpenedEvent {
        ItemOpenedEvent {
            header: self.header(1),
            item: self.item(),
        }
    }

    fn note(&self) -> Note {
        Note {
            event: EventSeq::new(2),
            item: self.item_id.clone(),
            author: self.actor.clone(),
            body: TextBody::new("First implementation slice is the contract repo."),
        }
    }

    fn note_event(&self) -> NoteAddedEvent {
        NoteAddedEvent {
            header: self.header(2),
            note: self.note(),
        }
    }

    fn edge(&self) -> Edge {
        Edge {
            event: EventSeq::new(3),
            source: StableItemId::new("item-0000000000000002"),
            kind: EdgeKind::DependsOn,
            target: EdgeTarget::Item(self.item_id.clone()),
            body: Some(TextBody::new("Implementation waits on the contract.")),
        }
    }

    fn edge_event(&self) -> EdgeAddedEvent {
        EdgeAddedEvent {
            header: self.header(3),
            edge: self.edge(),
        }
    }

    fn status_event(&self) -> StatusChangedEvent {
        StatusChangedEvent {
            header: self.header(4),
            item: self.item_id.clone(),
            status: ItemStatus::Closed,
            body: Some(TextBody::new("Contract shipped.")),
        }
    }

    fn alias_event(&self) -> AliasAddedEvent {
        AliasAddedEvent {
            header: self.header(5),
            item: self.item_id.clone(),
            alias: ExternalAlias::new("primary-9iv"),
        }
    }

    fn view(&self) -> View {
        View {
            items: vec![self.item()],
            edges: vec![self.edge()],
            notes: vec![self.note()],
            events: vec![
                Event::ItemOpened(self.opened_event()),
                Event::NoteAdded(self.note_event()),
                Event::EdgeAdded(self.edge_event()),
                Event::StatusChanged(self.status_event()),
                Event::AliasAdded(self.alias_event()),
            ],
        }
    }

    fn assert_request_round_trips(&self, request: MindRequest) {
        let decoded = round_trip_request(request.clone());
        assert_eq!(decoded, request);
    }
}

// ─── Request variants ─────────────────────────────────────

#[test]
fn role_claim_with_paths_round_trips() {
    let request = MindRequest::RoleClaim(RoleClaim {
        role: RoleName::Designer,
        scopes: vec![sample_path_scope(), sample_task_scope()],
        reason: sample_reason(),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn role_name_covers_workspace_coordination_roles() {
    for role in RoleName::ALL {
        let request = MindRequest::RoleClaim(RoleClaim {
            role,
            scopes: vec![sample_path_scope()],
            reason: sample_reason(),
        });
        let decoded = round_trip_request(request.clone());
        assert_eq!(decoded, request);
    }
}

#[test]
fn role_name_parses_workspace_coordination_tokens() {
    let cases = [
        ("operator", RoleName::Operator),
        ("operator-assistant", RoleName::OperatorAssistant),
        ("designer", RoleName::Designer),
        ("designer-assistant", RoleName::DesignerAssistant),
        ("system-specialist", RoleName::SystemSpecialist),
        ("system-assistant", RoleName::SystemAssistant),
        ("poet", RoleName::Poet),
        ("poet-assistant", RoleName::PoetAssistant),
    ];

    for (token, role) in cases {
        assert_eq!(RoleName::from_wire_token(token), Ok(role));
        assert_eq!(token.parse::<RoleName>(), Ok(role));
        assert_eq!(role.as_wire_token(), token);
        assert_eq!(role.to_string(), token);
    }
}

#[test]
fn role_name_rejects_unregistered_workspace_roles() {
    assert!(RoleName::from_wire_token("").is_err());
    assert!(RoleName::from_wire_token("operator assistant").is_err());
    assert!(RoleName::from_wire_token("Operator").is_err());
    assert!(RoleName::from_wire_token("critic").is_err());
}

#[test]
fn role_release_round_trips() {
    let request = MindRequest::RoleRelease(RoleRelease {
        role: RoleName::Operator,
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn role_handoff_round_trips() {
    let request = MindRequest::RoleHandoff(RoleHandoff {
        from: RoleName::Designer,
        to: RoleName::Operator,
        scopes: vec![sample_path_scope()],
        reason: ScopeReason::from_text("router migration handoff").expect("scope reason"),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn role_observation_round_trips() {
    let request = MindRequest::RoleObservation(RoleObservation);
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_submission_round_trips() {
    let request = MindRequest::ActivitySubmission(ActivitySubmission {
        role: RoleName::OperatorAssistant,
        scope: sample_path_scope(),
        reason: ScopeReason::from_text("audit signal-persona-system integration")
            .expect("scope reason"),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_unfiltered_round_trips() {
    let request = MindRequest::ActivityQuery(ActivityQuery {
        limit: 25,
        filters: vec![],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_with_role_filter_round_trips() {
    let request = MindRequest::ActivityQuery(ActivityQuery {
        limit: 50,
        filters: vec![ActivityFilter::RoleFilter(RoleName::Operator)],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_with_path_prefix_round_trips() {
    let request = MindRequest::ActivityQuery(ActivityQuery {
        limit: 10,
        filters: vec![ActivityFilter::PathPrefix(
            WirePath::from_absolute_path("/git/github.com/LiGoldragon/persona-router")
                .expect("absolute path"),
        )],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_with_task_filter_round_trips() {
    let request = MindRequest::ActivityQuery(ActivityQuery {
        limit: 100,
        filters: vec![ActivityFilter::TaskToken(sample_task())],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn open_request_round_trips_through_length_prefixed_frame() {
    let request = MindRequest::Opening(Opening {
        kind: ItemKind::Task,
        priority: ItemPriority::High,
        title: Title::new("Replace BEADS"),
        body: TextBody::new("Open a typed mind item."),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn add_note_request_round_trips() {
    let request = MindRequest::NoteSubmission(NoteSubmission {
        item: ItemReference::Display(DisplayId::new("9iv")),
        body: TextBody::new("Append-only note."),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn link_request_round_trips_with_typed_edge_kind() {
    let request = MindRequest::Link(Link {
        source: ItemReference::Display(DisplayId::new("abc")),
        kind: EdgeKind::DependsOn,
        target: LinkTarget::Item(ItemReference::Display(DisplayId::new("9iv"))),
        body: None,
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn link_request_round_trips_with_external_report_reference() {
    let request = MindRequest::Link(Link {
        source: ItemReference::Display(DisplayId::new("9iv")),
        kind: EdgeKind::References,
        target: LinkTarget::External(ExternalReference::Report(ReportPath::new(
            "reports/operator/100-persona-mind-central-rename-plan.md",
        ))),
        body: Some(TextBody::new("Research basis for this work item.")),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn status_change_request_round_trips() {
    let request = MindRequest::StatusChange(StatusChange {
        item: ItemReference::Alias(ExternalAlias::new("primary-9iv")),
        status: ItemStatus::InProgress,
        body: Some(TextBody::new("Operator started it.")),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn add_alias_request_round_trips() {
    let request = MindRequest::AliasAssignment(AliasAssignment {
        item: ItemReference::Stable(StableItemId::new("item-0000000000000001")),
        alias: ExternalAlias::new("primary-9iv"),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn every_query_kind_round_trips() {
    let fixture = MemoryFixture::new();
    let kinds = vec![
        QueryKind::Ready,
        QueryKind::Blocked,
        QueryKind::Open,
        QueryKind::RecentEvents,
        QueryKind::ByItem(ItemReference::Stable(fixture.item_id.clone())),
        QueryKind::ByKind(ItemKind::Decision),
        QueryKind::ByStatus(ItemStatus::Closed),
        QueryKind::ByAlias(ExternalAlias::new("primary-9iv")),
    ];

    for kind in kinds {
        fixture.assert_request_round_trips(MindRequest::Query(Query {
            kind,
            limit: QueryLimit::new(25),
        }));
    }
}

#[test]
fn every_edge_kind_round_trips_as_a_link_request() {
    let fixture = MemoryFixture::new();
    let kinds = vec![
        EdgeKind::DependsOn,
        EdgeKind::ParentOf,
        EdgeKind::RelatesTo,
        EdgeKind::Duplicates,
        EdgeKind::Supersedes,
        EdgeKind::Answers,
        EdgeKind::References,
    ];

    for kind in kinds {
        fixture.assert_request_round_trips(MindRequest::Link(Link {
            source: ItemReference::Stable(StableItemId::new("item-0000000000000002")),
            kind,
            target: LinkTarget::Item(ItemReference::Stable(fixture.item_id.clone())),
            body: None,
        }));
    }
}

#[test]
fn every_external_reference_variant_round_trips_as_a_link_target() {
    let fixture = MemoryFixture::new();
    let targets = vec![
        ExternalReference::Report(ReportPath::new("reports/operator/100-mind.md")),
        ExternalReference::GitCommit(CommitHash::new("7f0bf022")),
        ExternalReference::BeadsTask(BeadsToken::new("primary-9iv")),
        ExternalReference::File(ReferencePath::new(
            "/git/github.com/LiGoldragon/persona-mind/src/lib.rs",
        )),
    ];

    for target in targets {
        fixture.assert_request_round_trips(MindRequest::Link(Link {
            source: ItemReference::Stable(fixture.item_id.clone()),
            kind: EdgeKind::References,
            target: LinkTarget::External(target),
            body: Some(TextBody::new("typed external reference")),
        }));
    }
}

// ─── Reply variants ───────────────────────────────────────

#[test]
fn claim_acceptance_round_trips() {
    let reply = MindReply::ClaimAcceptance(ClaimAcceptance {
        role: RoleName::Designer,
        scopes: vec![sample_path_scope()],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn claim_rejection_round_trips() {
    let reply = MindReply::ClaimRejection(ClaimRejection {
        role: RoleName::Designer,
        conflicts: vec![ScopeConflict {
            scope: sample_path_scope(),
            held_by: RoleName::Operator,
            held_reason: ScopeReason::from_text("Persona-prefix sweep").expect("scope reason"),
        }],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn release_acknowledgment_round_trips() {
    let reply = MindReply::ReleaseAcknowledgment(ReleaseAcknowledgment {
        role: RoleName::Designer,
        released_scopes: vec![sample_path_scope(), sample_task_scope()],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn handoff_acceptance_round_trips() {
    let reply = MindReply::HandoffAcceptance(HandoffAcceptance {
        from: RoleName::Designer,
        to: RoleName::Operator,
        scopes: vec![sample_path_scope()],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn handoff_rejection_source_does_not_hold_round_trips() {
    let reply = MindReply::HandoffRejection(HandoffRejection {
        from: RoleName::Designer,
        to: RoleName::Operator,
        reason: HandoffRejectionReason::SourceRoleDoesNotHold,
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn handoff_rejection_target_conflict_round_trips() {
    let reply = MindReply::HandoffRejection(HandoffRejection {
        from: RoleName::Designer,
        to: RoleName::Operator,
        reason: HandoffRejectionReason::TargetRoleConflict(vec![ScopeConflict {
            scope: sample_path_scope(),
            held_by: RoleName::DesignerAssistant,
            held_reason: ScopeReason::from_text("audit pass").expect("scope reason"),
        }]),
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn role_snapshot_round_trips() {
    let reply = MindReply::RoleSnapshot(RoleSnapshot {
        roles: vec![
            RoleStatus {
                role: RoleName::Designer,
                claims: vec![ClaimEntry {
                    scope: sample_path_scope(),
                    reason: sample_reason(),
                }],
            },
            RoleStatus {
                role: RoleName::Operator,
                claims: vec![],
            },
        ],
        recent_activity: vec![Activity {
            role: RoleName::Designer,
            scope: sample_path_scope(),
            reason: sample_reason(),
            stamped_at: TimestampNanos::new(1_730_000_000_000_000_000),
        }],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn activity_acknowledgment_round_trips() {
    let reply = MindReply::ActivityAcknowledgment(ActivityAcknowledgment { slot: 42 });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn activity_list_round_trips() {
    let reply = MindReply::ActivityList(ActivityList {
        records: vec![
            Activity {
                role: RoleName::Designer,
                scope: sample_path_scope(),
                reason: ScopeReason::from_text("rescope per /91 §3.1").expect("scope reason"),
                stamped_at: TimestampNanos::new(1_730_000_000_000_000_000),
            },
            Activity {
                role: RoleName::Operator,
                scope: sample_task_scope(),
                reason: ScopeReason::from_text("ractor adoption").expect("scope reason"),
                stamped_at: TimestampNanos::new(1_730_000_001_000_000_000),
            },
        ],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn memory_receipt_replies_round_trip() {
    let fixture = MemoryFixture::new();
    let replies = vec![
        MindReply::OpeningReceipt(OpeningReceipt {
            event: fixture.opened_event(),
        }),
        MindReply::NoteReceipt(NoteReceipt {
            event: fixture.note_event(),
        }),
        MindReply::LinkReceipt(LinkReceipt {
            event: fixture.edge_event(),
        }),
        MindReply::StatusReceipt(StatusReceipt {
            event: fixture.status_event(),
        }),
        MindReply::AliasReceipt(AliasReceipt {
            event: fixture.alias_event(),
        }),
        MindReply::View(fixture.view()),
        MindReply::Rejection(Rejection {
            reason: RejectionReason::UnknownItem,
        }),
    ];

    for reply in replies {
        let decoded = round_trip_reply(reply.clone());
        assert_eq!(decoded, reply);
    }
}

#[test]
fn from_impl_lifts_opening_into_request() {
    let opening = Opening {
        kind: ItemKind::Question,
        priority: ItemPriority::Normal,
        title: Title::new("Choose migration order"),
        body: TextBody::new("Need a decision before implementation."),
    };
    let request: MindRequest = opening.clone().into();
    assert_eq!(request, MindRequest::Opening(opening));
}

#[test]
fn from_impl_lifts_view_into_reply() {
    let view = MemoryFixture::new().view();
    let reply: MindReply = view.clone().into();
    assert_eq!(reply, MindReply::View(view));
}

// ─── Scope-reference variants ─────────────────────────────

#[test]
fn path_scope_round_trips() {
    let request = MindRequest::RoleClaim(RoleClaim {
        role: RoleName::Designer,
        scopes: vec![ScopeReference::Path(sample_path())],
        reason: sample_reason(),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn task_scope_round_trips() {
    let request = MindRequest::RoleClaim(RoleClaim {
        role: RoleName::Designer,
        scopes: vec![ScopeReference::Task(sample_task())],
        reason: sample_reason(),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

// ─── Boundary validation ──────────────────────────────────

#[test]
fn wire_path_requires_absolute_normalized_path() {
    assert!(WirePath::from_absolute_path("/git/github.com/LiGoldragon/persona").is_ok());
    assert!(WirePath::from_absolute_path("relative/path").is_err());
    assert!(WirePath::from_absolute_path("").is_err());
    assert!(WirePath::from_absolute_path("/git/../persona").is_err());

    let normalized = WirePath::from_absolute_path("/git//github.com/./LiGoldragon/persona/")
        .expect("normalizable absolute path");
    assert_eq!(normalized.as_str(), "/git/github.com/LiGoldragon/persona");

    let root = WirePath::from_absolute_path("/").expect("root path");
    assert_eq!(root.as_str(), "/");
}

#[test]
fn task_token_rejects_brackets_empty_and_whitespace() {
    assert!(TaskToken::from_wire_token("primary-f99").is_ok());
    assert!(TaskToken::from_wire_token("[primary-f99]").is_err());
    assert!(TaskToken::from_wire_token("").is_err());
    assert!(TaskToken::from_wire_token("primary f99").is_err());
}

#[test]
fn scope_reason_rejects_blank_and_multiline_text() {
    assert!(ScopeReason::from_text("short reason").is_ok());
    assert!(ScopeReason::from_text("").is_err());
    assert!(ScopeReason::from_text("   ").is_err());
    assert!(ScopeReason::from_text("first\nsecond").is_err());
}
