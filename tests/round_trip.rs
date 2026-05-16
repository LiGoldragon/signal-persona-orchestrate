//! Architectural-truth round-trip tests for the
//! `signal-persona-mind` channel.
//!
//! Per `~/primary/skills/architectural-truth-tests.md`,
//! each variant of both enums has a witness test that
//! proves the macro-emitted type round-trips through a
//! length-prefixed Frame.

use nota_codec::{Decoder, Encoder, Error as NotaError, NotaDecode, NotaEncode};
use signal_core::{
    ExchangeIdentifier, ExchangeLane, LaneSequence, NonEmpty, Reply, RequestPayload, SessionEpoch,
    SignalVerb, StreamEventIdentifier, SubReply, SubscriptionTokenInner,
};
use signal_persona_auth::{ChannelId, ComponentName, ConnectionClass, MessageOrigin};
use signal_persona_mind::*;

// ─── Helpers ──────────────────────────────────────────────

fn exchange() -> ExchangeIdentifier {
    ExchangeIdentifier::new(
        SessionEpoch::new(1),
        ExchangeLane::Connector,
        LaneSequence::first(),
    )
}

fn stream_event() -> StreamEventIdentifier {
    StreamEventIdentifier::new(
        SessionEpoch::new(1),
        ExchangeLane::Acceptor,
        LaneSequence::first(),
    )
}

fn round_trip_request(request: MindRequest) -> MindRequest {
    let expected_verb = request.signal_verb();
    let frame = MindFrame::new(MindFrameBody::Request {
        exchange: exchange(),
        request: request.into_request(),
    });
    let bytes = frame.encode_length_prefixed().expect("encode");
    let decoded = MindFrame::decode_length_prefixed(&bytes).expect("decode");
    match decoded.into_body() {
        MindFrameBody::Request { request, .. } => {
            let operation = request.operations().head();
            assert_eq!(operation.verb, expected_verb);
            operation.payload.clone()
        }
        other => panic!("expected request operation, got {other:?}"),
    }
}

fn round_trip_reply(reply: MindReply) -> MindReply {
    let frame = MindFrame::new(MindFrameBody::Reply {
        exchange: exchange(),
        reply: Reply::completed(NonEmpty::single(SubReply::Ok {
            verb: SignalVerb::Match,
            payload: reply,
        })),
    });
    let bytes = frame.encode_length_prefixed().expect("encode");
    let decoded = MindFrame::decode_length_prefixed(&bytes).expect("decode");
    match decoded.into_body() {
        MindFrameBody::Reply { reply, .. } => match reply {
            Reply::Accepted { per_operation, .. } => match per_operation.into_head() {
                SubReply::Ok { payload, .. } => payload,
                other => panic!("expected accepted reply payload, got {other:?}"),
            },
            other => panic!("expected accepted reply, got {other:?}"),
        },
        other => panic!("expected reply operation, got {other:?}"),
    }
}

fn round_trip_event(event: MindEvent) -> MindEvent {
    let frame = MindFrame::new(MindFrameBody::SubscriptionEvent {
        event_identifier: stream_event(),
        token: SubscriptionTokenInner::new(1),
        event,
    });
    let bytes = frame.encode_length_prefixed().expect("encode");
    let decoded = MindFrame::decode_length_prefixed(&bytes).expect("decode");
    match decoded.into_body() {
        MindFrameBody::SubscriptionEvent { event, .. } => event,
        other => panic!("expected subscription event, got {other:?}"),
    }
}

fn round_trip_nota<T>(value: T, expected: &str)
where
    T: NotaEncode + NotaDecode + PartialEq + std::fmt::Debug,
{
    let mut encoder = Encoder::new();
    value.encode(&mut encoder).expect("encode nota text");
    let encoded = encoder.into_string();
    assert_eq!(encoded, expected);

    let mut decoder = Decoder::new(&encoded);
    let recovered = T::decode(&mut decoder).expect("decode nota text");
    assert_eq!(recovered, value);
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

fn sample_adjudication_request() -> AdjudicationRequestId {
    AdjudicationRequestId::new("aab")
}

fn sample_channel() -> ChannelId {
    ChannelId::new("channel-aab")
}

fn sample_record() -> RecordId {
    RecordId::new("rec-aab")
}

fn sample_relation() -> RelationId {
    RelationId::new("rel-aab")
}

fn sample_engine() -> signal_persona_auth::EngineId {
    signal_persona_auth::EngineId::new("engine-aab")
}

fn sample_actor() -> ActorName {
    ActorName::new("operator")
}

fn sample_internal_endpoint(component: ComponentName) -> ChannelEndpoint {
    ChannelEndpoint::Internal(component)
}

fn sample_external_owner_endpoint() -> ChannelEndpoint {
    ChannelEndpoint::External(ConnectionClass::Owner)
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
            item_id: StableItemId::new("aab"),
            display_id: DisplayId::new("aab"),
            actor: ActorName::new("operator"),
            operation: OperationId::new("aab"),
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
            aliases: vec![ExternalAlias::new("primary-aab")],
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
            source: StableItemId::new("aac"),
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
            alias: ExternalAlias::new("primary-aab"),
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

struct MindGraphFixture {
    record: RecordId,
    relation: RelationId,
    actor: ActorName,
    occurred_at: TimestampNanos,
}

impl MindGraphFixture {
    fn new() -> Self {
        Self {
            record: sample_record(),
            relation: sample_relation(),
            actor: sample_actor(),
            occurred_at: TimestampNanos::new(1_790_000_000_000_000_000),
        }
    }

    fn observation_body(&self) -> ThoughtBody {
        ThoughtBody::Observation(ObservationBody {
            summary: ObservationSummary::ComponentReady(ComponentReady {
                component: ComponentName::Mind,
                engine: sample_engine(),
            }),
            detail: Some(TextBody::new("mind graph contract ready")),
            location: None,
        })
    }

    fn thought(&self) -> Thought {
        Thought {
            id: self.record.clone(),
            kind: ThoughtKind::Observation,
            body: self.observation_body(),
            author: self.actor.clone(),
            occurred_at: self.occurred_at,
        }
    }

    fn identity_reference_thought(&self) -> Thought {
        Thought {
            id: RecordId::new("identity-aab"),
            kind: ThoughtKind::Reference,
            body: self.reference_body(),
            author: self.actor.clone(),
            occurred_at: self.occurred_at,
        }
    }

    fn file_reference_thought(&self) -> Thought {
        Thought {
            id: RecordId::new("file-aab"),
            kind: ThoughtKind::Reference,
            body: ThoughtBody::Reference(ReferenceBody {
                target: ReferenceTarget::File(FileReference {
                    path: sample_path(),
                }),
                sense: Some(TextBody::new("a source file is not an identity")),
            }),
            author: self.actor.clone(),
            occurred_at: self.occurred_at,
        }
    }

    fn relation(&self) -> Relation {
        Relation {
            id: self.relation.clone(),
            kind: RelationKind::Authored,
            source: RecordId::new("identity-aab"),
            target: self.record.clone(),
            author: self.actor.clone(),
            occurred_at: self.occurred_at,
            note: Some(TextBody::new("operator authored the thought")),
        }
    }

    fn decision_body(&self) -> ThoughtBody {
        ThoughtBody::Decision(DecisionBody {
            question: TextBody::new("Should the mind graph land in the contract first?"),
            alternatives: vec![
                Alternative {
                    id: AlternativeId::new("contract-first"),
                    description: TextBody::new("Land signal-persona-mind first."),
                    pros: vec![TextBody::new("consumers compile against one vocabulary")],
                    cons: vec![TextBody::new("persona-mind waits for the pin")],
                },
                Alternative {
                    id: AlternativeId::new("consumer-first"),
                    description: TextBody::new("Prototype in persona-mind first."),
                    pros: vec![TextBody::new("fast local reducer feedback")],
                    cons: vec![TextBody::new("risks a parallel vocabulary")],
                },
            ],
            chosen: AlternativeId::new("contract-first"),
            criteria: vec![TextBody::new("contracts choreograph parallel work")],
            rationale: TextBody::new("The typed vocabulary must be the shared boundary."),
        })
    }

    fn reference_body(&self) -> ThoughtBody {
        ThoughtBody::Reference(ReferenceBody {
            target: ReferenceTarget::Identity(IdentityReference::Component(ComponentIdentity {
                engine: sample_engine(),
                component: ComponentName::Mind,
            })),
            sense: Some(TextBody::new("the component whose graph owns this record")),
        })
    }
}

// ─── Mind graph contract variants ─────────────────────────

#[test]
fn every_thought_kind_round_trips_through_nota_text() {
    let cases = [
        (ThoughtKind::Observation, "Observation"),
        (ThoughtKind::Memory, "Memory"),
        (ThoughtKind::Belief, "Belief"),
        (ThoughtKind::Goal, "Goal"),
        (ThoughtKind::Claim, "Claim"),
        (ThoughtKind::Decision, "Decision"),
        (ThoughtKind::Reference, "Reference"),
    ];

    for (kind, expected) in cases {
        round_trip_nota(kind, expected);
    }
}

#[test]
fn every_relation_kind_round_trips_through_nota_text() {
    let cases = [
        (RelationKind::Implements, "Implements"),
        (RelationKind::Realizes, "Realizes"),
        (RelationKind::Requires, "Requires"),
        (RelationKind::Supports, "Supports"),
        (RelationKind::Refutes, "Refutes"),
        (RelationKind::Supersedes, "Supersedes"),
        (RelationKind::Authored, "Authored"),
        (RelationKind::References, "References"),
        (RelationKind::Decides, "Decides"),
        (RelationKind::Considered, "Considered"),
        (RelationKind::Belongs, "Belongs"),
    ];

    for (kind, expected) in cases {
        round_trip_nota(kind, expected);
    }
}

#[test]
fn relation_kind_domain_table_covers_every_relation_kind() {
    let valid_cases = [
        (
            RelationKind::Implements,
            ThoughtKind::Claim,
            ThoughtKind::Goal,
        ),
        (
            RelationKind::Realizes,
            ThoughtKind::Observation,
            ThoughtKind::Claim,
        ),
        (RelationKind::Requires, ThoughtKind::Goal, ThoughtKind::Goal),
        (
            RelationKind::Requires,
            ThoughtKind::Claim,
            ThoughtKind::Claim,
        ),
        (
            RelationKind::Supports,
            ThoughtKind::Observation,
            ThoughtKind::Belief,
        ),
        (
            RelationKind::Supports,
            ThoughtKind::Belief,
            ThoughtKind::Belief,
        ),
        (
            RelationKind::Refutes,
            ThoughtKind::Observation,
            ThoughtKind::Belief,
        ),
        (
            RelationKind::Refutes,
            ThoughtKind::Belief,
            ThoughtKind::Belief,
        ),
        (
            RelationKind::Supersedes,
            ThoughtKind::Decision,
            ThoughtKind::Decision,
        ),
        (
            RelationKind::Authored,
            ThoughtKind::Reference,
            ThoughtKind::Observation,
        ),
        (
            RelationKind::References,
            ThoughtKind::Belief,
            ThoughtKind::Reference,
        ),
        (
            RelationKind::Decides,
            ThoughtKind::Decision,
            ThoughtKind::Goal,
        ),
        (
            RelationKind::Considered,
            ThoughtKind::Decision,
            ThoughtKind::Belief,
        ),
        (
            RelationKind::Belongs,
            ThoughtKind::Observation,
            ThoughtKind::Memory,
        ),
        (RelationKind::Belongs, ThoughtKind::Claim, ThoughtKind::Goal),
    ];

    for relation in RelationKind::ALL {
        assert!(
            valid_cases
                .iter()
                .any(|(candidate, _, _)| *candidate == relation),
            "{relation:?} must have at least one valid witness case",
        );
    }

    for (relation, source, target) in valid_cases {
        relation
            .validate_endpoint_kinds(source, target)
            .unwrap_or_else(|mismatch| panic!("unexpected mismatch: {mismatch:?}"));
    }
}

#[test]
fn relation_kind_rejects_wrong_domain() {
    let mismatch = RelationKind::Implements
        .validate_endpoint_kinds(ThoughtKind::Goal, ThoughtKind::Claim)
        .expect_err("Goal -> Claim cannot implement");

    assert_eq!(mismatch.relation, RelationKind::Implements);
    assert_eq!(mismatch.reason, RelationKindMismatchReason::DomainRange);
    assert_eq!(mismatch.expected_source_kinds, vec![ThoughtKind::Claim]);
    assert_eq!(mismatch.expected_target_kinds, vec![ThoughtKind::Goal]);
    assert_eq!(mismatch.got_source_kind, ThoughtKind::Goal);
    assert_eq!(mismatch.got_target_kind, ThoughtKind::Claim);
}

#[test]
fn authored_relation_rejects_non_identity_reference_source() {
    let fixture = MindGraphFixture::new();
    let source = fixture.file_reference_thought();
    let target = fixture.thought();
    let mismatch = RelationKind::Authored
        .validate_endpoints(&source, &target)
        .expect_err("Authored source must be an identity reference");

    assert_eq!(mismatch.relation, RelationKind::Authored);
    assert_eq!(
        mismatch.reason,
        RelationKindMismatchReason::AuthoredSourceNotIdentity
    );
    assert_eq!(mismatch.expected_source_kinds, vec![ThoughtKind::Reference]);
    assert_eq!(mismatch.got_source_kind, ThoughtKind::Reference);
    assert_eq!(mismatch.got_target_kind, ThoughtKind::Observation);
}

#[test]
fn authored_relation_accepts_identity_reference_source() {
    let fixture = MindGraphFixture::new();
    RelationKind::Authored
        .validate_endpoints(&fixture.identity_reference_thought(), &fixture.thought())
        .expect("identity reference can author any thought");
}

#[test]
fn submit_thought_request_round_trips() {
    let fixture = MindGraphFixture::new();
    let request = MindRequest::SubmitThought(SubmitThought {
        kind: ThoughtKind::Observation,
        body: fixture.observation_body(),
    });

    assert_eq!(round_trip_request(request.clone()), request);
}

#[test]
fn submit_relation_request_round_trips() {
    let request = MindRequest::SubmitRelation(SubmitRelation {
        kind: RelationKind::Implements,
        source: RecordId::new("claim-aab"),
        target: RecordId::new("goal-aab"),
        note: Some(TextBody::new("claim commits work toward the goal")),
    });

    assert_eq!(round_trip_request(request.clone()), request);
}

#[test]
fn query_thoughts_request_round_trips_with_composite_filter() {
    let request = MindRequest::QueryThoughts(QueryThoughts {
        filter: ThoughtFilter::Composite(CompositeThoughtFilter {
            kinds: vec![ThoughtKind::Goal, ThoughtKind::Claim],
            author: Some(sample_actor()),
            time_range: None,
            goal: None,
            memory: None,
        }),
        limit: 32,
    });

    assert_eq!(round_trip_request(request.clone()), request);
}

#[test]
fn query_relations_request_round_trips_with_source_filter() {
    let request = MindRequest::QueryRelations(QueryRelations {
        filter: RelationFilter::BySource(ByRelationSource {
            source: RecordId::new("goal-aab"),
        }),
        limit: 16,
    });

    assert_eq!(round_trip_request(request.clone()), request);
}

#[test]
fn subscribe_requests_round_trip() {
    let thoughts = MindRequest::SubscribeThoughts(SubscribeThoughts {
        filter: ThoughtFilter::InMemory(InMemory {
            memory: RecordId::new("memory-aab"),
        }),
    });
    let relations = MindRequest::SubscribeRelations(SubscribeRelations {
        filter: RelationFilter::ByTarget(ByRelationTarget {
            target: RecordId::new("goal-aab"),
        }),
    });

    assert_eq!(round_trip_request(thoughts.clone()), thoughts);
    assert_eq!(round_trip_request(relations.clone()), relations);
}

#[test]
fn thought_and_relation_replies_round_trip() {
    let fixture = MindGraphFixture::new();
    let replies = vec![
        MindReply::ThoughtCommitted(ThoughtCommitted {
            record: fixture.record.clone(),
            display: DisplayId::new("aab"),
            occurred_at: fixture.occurred_at,
        }),
        MindReply::RelationCommitted(RelationCommitted {
            relation: fixture.relation.clone(),
            occurred_at: fixture.occurred_at,
        }),
        MindReply::ThoughtList(ThoughtList {
            thoughts: vec![fixture.thought()],
            has_more: false,
        }),
        MindReply::RelationList(RelationList {
            relations: vec![fixture.relation()],
            has_more: false,
        }),
    ];

    for reply in replies {
        assert_eq!(round_trip_reply(reply.clone()), reply);
    }
}

#[test]
fn subscription_replies_round_trip() {
    let fixture = MindGraphFixture::new();
    let accepted = MindReply::SubscriptionAccepted(SubscriptionAccepted {
        subscription: SubscriptionId::new("sub-aab"),
        initial_snapshot: vec![
            MindSnapshot::Thought(fixture.thought()),
            MindSnapshot::Relation(fixture.relation()),
        ],
    });
    let retracted = MindReply::SubscriptionRetracted(SubscriptionRetracted {
        subscription: SubscriptionId::new("sub-aab"),
    });
    let event = MindEvent::SubscriptionDelta(SubscriptionEvent {
        subscription: SubscriptionId::new("sub-aab"),
        delta: MindDelta::ThoughtCommitted(Thought {
            body: fixture.decision_body(),
            kind: ThoughtKind::Decision,
            ..fixture.thought()
        }),
    });

    assert_eq!(round_trip_reply(accepted.clone()), accepted);
    assert_eq!(round_trip_reply(retracted.clone()), retracted);
    assert_eq!(round_trip_event(event.clone()), event);
    assert_eq!(event.stream_kind(), MindStreamKind::MindEventStream);
}

/// The streaming subscription contract pairs `Subscribe*` (opens) with
/// `SubscriptionRetraction` (close request) and `SubscriptionRetracted`
/// (final ack reply). The `signal_channel!` macro emits the
/// `opened_stream()` and `closed_stream()` discriminants from that
/// pairing; this test pins both halves so a future refactor that drops
/// the request-side retract verb in favor of a producer-only close
/// breaks compilation and review.
#[test]
fn subscribe_opens_and_subscription_retraction_closes_the_mind_event_stream() {
    let subscribe_thoughts = MindRequest::SubscribeThoughts(SubscribeThoughts {
        filter: ThoughtFilter::InMemory(InMemory {
            memory: RecordId::new("memory-aab"),
        }),
    });
    let subscribe_relations = MindRequest::SubscribeRelations(SubscribeRelations {
        filter: RelationFilter::ByTarget(ByRelationTarget {
            target: RecordId::new("goal-aab"),
        }),
    });
    let retract = MindRequest::SubscriptionRetraction(SubscriptionId::new("sub-aab"));

    assert_eq!(
        subscribe_thoughts.opened_stream(),
        Some(MindStreamKind::MindEventStream),
    );
    assert_eq!(
        subscribe_relations.opened_stream(),
        Some(MindStreamKind::MindEventStream),
    );
    assert_eq!(
        retract.closed_stream(),
        Some(MindStreamKind::MindEventStream),
    );
    assert_eq!(subscribe_thoughts.closed_stream(), None);
    assert_eq!(retract.opened_stream(), None);

    assert_eq!(round_trip_request(retract.clone()), retract);
}

#[test]
fn reference_identity_thought_body_round_trips() {
    let fixture = MindGraphFixture::new();
    let request = MindRequest::SubmitThought(SubmitThought {
        kind: ThoughtKind::Reference,
        body: fixture.reference_body(),
    });

    assert_eq!(round_trip_request(request.clone()), request);
}

#[test]
fn unimplemented_reply_round_trips_as_typed_reply() {
    let cases = [
        (
            MindUnimplementedReason::NotInPrototypeScope,
            "(MindRequestUnimplemented (NotInPrototypeScope))",
        ),
        (
            MindUnimplementedReason::ChoreographyPolicyMissing,
            "(MindRequestUnimplemented (ChoreographyPolicyMissing))",
        ),
        (
            MindUnimplementedReason::DependencyMissing(DependencyKind::Router),
            "(MindRequestUnimplemented (DependencyMissing Router))",
        ),
        (
            MindUnimplementedReason::ResourceUnavailable(ResourceKind::Database),
            "(MindRequestUnimplemented (ResourceUnavailable Database))",
        ),
    ];

    for (reason, expected_text) in cases {
        let reply = MindReply::MindRequestUnimplemented(MindRequestUnimplemented { reason });
        assert_eq!(round_trip_reply(reply.clone()), reply);
        round_trip_nota(reply, expected_text);
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
fn role_claim_request_round_trips_through_nota_text() {
    round_trip_nota(
        MindRequest::RoleClaim(RoleClaim {
            role: RoleName::Designer,
            scopes: vec![sample_path_scope(), sample_task_scope()],
            reason: sample_reason(),
        }),
        "(RoleClaim Designer [(Path \"/git/github.com/LiGoldragon/signal-persona-mind/src/lib.rs\") (Task primary-f99)] \"design-cascade per /93\")",
    );
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
        item: ItemReference::Display(DisplayId::new("aab")),
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
        target: LinkTarget::Item(ItemReference::Display(DisplayId::new("aab"))),
        body: None,
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn link_request_round_trips_with_external_report_reference() {
    let request = MindRequest::Link(Link {
        source: ItemReference::Display(DisplayId::new("aab")),
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
        item: ItemReference::Alias(ExternalAlias::new("primary-aab")),
        status: ItemStatus::InProgress,
        body: Some(TextBody::new("Operator started it.")),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn add_alias_request_round_trips() {
    let request = MindRequest::AliasAssignment(AliasAssignment {
        item: ItemReference::Stable(StableItemId::new("aab")),
        alias: ExternalAlias::new("primary-aab"),
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
        QueryKind::ByAlias(ExternalAlias::new("primary-aab")),
    ];

    for kind in kinds {
        fixture.assert_request_round_trips(MindRequest::Query(Query {
            kind,
            limit: QueryLimit::new(25),
        }));
    }
}

#[test]
fn query_request_round_trips_through_nota_text() {
    round_trip_nota(
        MindRequest::Query(Query {
            kind: QueryKind::Ready,
            limit: QueryLimit::new(25),
        }),
        "(Query (Ready) 25)",
    );
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
            source: ItemReference::Stable(StableItemId::new("aac")),
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
        ExternalReference::BeadsTask(BeadsToken::new("primary-aab")),
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

#[test]
fn adjudication_request_round_trips() {
    let request = MindRequest::AdjudicationRequest(AdjudicationRequest {
        request: sample_adjudication_request(),
        origin: MessageOrigin::External(ConnectionClass::Owner),
        destination: sample_internal_endpoint(ComponentName::Router),
        kind: ChannelMessageKind::MessageSubmission,
        body_summary: TextBody::new("owner asks router to deliver a prompt"),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn channel_choreography_requests_round_trip() {
    let requests = vec![
        MindRequest::ChannelGrant(ChannelGrant {
            source: sample_external_owner_endpoint(),
            destination: sample_internal_endpoint(ComponentName::Router),
            kinds: vec![
                ChannelMessageKind::MessageSubmission,
                ChannelMessageKind::InboxQuery,
            ],
            duration: ChannelDuration::Permanent,
        }),
        MindRequest::ChannelExtend(ChannelExtend {
            channel: sample_channel(),
            duration: ChannelDuration::TimeBound(TimestampNanos::new(1_730_000_000_000_000_000)),
        }),
        MindRequest::ChannelRetract(ChannelRetract {
            channel: sample_channel(),
            reason: TextBody::new("operator closed the path"),
        }),
        MindRequest::AdjudicationDeny(AdjudicationDeny {
            request: sample_adjudication_request(),
            reason: TextBody::new("destination is unavailable"),
        }),
        MindRequest::ChannelList(ChannelList {
            filters: vec![
                ChannelFilter::Source(sample_internal_endpoint(ComponentName::Mind)),
                ChannelFilter::Destination(sample_internal_endpoint(ComponentName::Router)),
                ChannelFilter::Kind(ChannelMessageKind::ChannelGrant),
            ],
        }),
    ];

    for request in requests {
        let decoded = round_trip_request(request.clone());
        assert_eq!(decoded, request);
    }
}

#[test]
fn channel_grant_request_round_trips_through_nota_text() {
    round_trip_nota(
        MindRequest::ChannelGrant(ChannelGrant {
            source: sample_external_owner_endpoint(),
            destination: sample_internal_endpoint(ComponentName::Router),
            kinds: vec![
                ChannelMessageKind::MessageSubmission,
                ChannelMessageKind::InboxQuery,
            ],
            duration: ChannelDuration::Permanent,
        }),
        "(ChannelGrant (External (Owner)) (Internal Router) [MessageSubmission InboxQuery] (Permanent))",
    );
}

#[test]
fn message_ingress_kind_is_distinct_from_generic_message_submission() {
    assert_ne!(
        ChannelMessageKind::MessageIngressSubmission,
        ChannelMessageKind::MessageSubmission
    );
    round_trip_nota(
        ChannelMessageKind::MessageIngressSubmission,
        "MessageIngressSubmission",
    );
    round_trip_nota(ChannelMessageKind::MessageSubmission, "MessageSubmission");
}

#[test]
fn mind_request_exposes_contract_owned_operation_kind() {
    let fixture = MemoryFixture::new();
    let cases = vec![
        (
            MindRequest::SubmitThought(SubmitThought {
                kind: ThoughtKind::Observation,
                body: MindGraphFixture::new().observation_body(),
            }),
            MindOperationKind::SubmitThought,
        ),
        (
            MindRequest::SubmitRelation(SubmitRelation {
                kind: RelationKind::Implements,
                source: RecordId::new("claim-aab"),
                target: RecordId::new("goal-aab"),
                note: None,
            }),
            MindOperationKind::SubmitRelation,
        ),
        (
            MindRequest::QueryThoughts(QueryThoughts {
                filter: ThoughtFilter::ByKind(ByThoughtKind {
                    kinds: vec![ThoughtKind::Goal],
                }),
                limit: 10,
            }),
            MindOperationKind::QueryThoughts,
        ),
        (
            MindRequest::QueryRelations(QueryRelations {
                filter: RelationFilter::ByKind(ByRelationKind {
                    kinds: vec![RelationKind::Implements],
                }),
                limit: 10,
            }),
            MindOperationKind::QueryRelations,
        ),
        (
            MindRequest::SubscribeThoughts(SubscribeThoughts {
                filter: ThoughtFilter::ByAuthor(ByThoughtAuthor {
                    author: ActorName::new("operator"),
                }),
            }),
            MindOperationKind::SubscribeThoughts,
        ),
        (
            MindRequest::SubscribeRelations(SubscribeRelations {
                filter: RelationFilter::ByTarget(ByRelationTarget {
                    target: RecordId::new("goal-aab"),
                }),
            }),
            MindOperationKind::SubscribeRelations,
        ),
        (
            MindRequest::RoleClaim(RoleClaim {
                role: RoleName::Designer,
                scopes: vec![sample_path_scope()],
                reason: sample_reason(),
            }),
            MindOperationKind::RoleClaim,
        ),
        (
            MindRequest::RoleRelease(RoleRelease {
                role: RoleName::Designer,
            }),
            MindOperationKind::RoleRelease,
        ),
        (
            MindRequest::RoleHandoff(RoleHandoff {
                from: RoleName::Designer,
                to: RoleName::Operator,
                scopes: vec![sample_path_scope()],
                reason: sample_reason(),
            }),
            MindOperationKind::RoleHandoff,
        ),
        (
            MindRequest::RoleObservation(RoleObservation),
            MindOperationKind::RoleObservation,
        ),
        (
            MindRequest::ActivitySubmission(ActivitySubmission {
                role: RoleName::Operator,
                scope: sample_path_scope(),
                reason: sample_reason(),
            }),
            MindOperationKind::ActivitySubmission,
        ),
        (
            MindRequest::ActivityQuery(ActivityQuery {
                limit: 10,
                filters: vec![ActivityFilter::RoleFilter(RoleName::Operator)],
            }),
            MindOperationKind::ActivityQuery,
        ),
        (
            MindRequest::Opening(Opening {
                kind: ItemKind::Task,
                priority: ItemPriority::High,
                title: Title::new("Add operation kinds"),
                body: TextBody::new("Expose discriminants from the contract crate."),
            }),
            MindOperationKind::Opening,
        ),
        (
            MindRequest::NoteSubmission(NoteSubmission {
                item: ItemReference::Stable(fixture.item_id.clone()),
                body: TextBody::new("Contract-owned discriminant witness."),
            }),
            MindOperationKind::NoteSubmission,
        ),
        (
            MindRequest::Link(Link {
                source: ItemReference::Stable(fixture.item_id.clone()),
                kind: EdgeKind::References,
                target: LinkTarget::External(ExternalReference::File(ReferencePath::new(
                    "/git/github.com/LiGoldragon/signal-persona-mind/src/lib.rs",
                ))),
                body: None,
            }),
            MindOperationKind::Link,
        ),
        (
            MindRequest::StatusChange(StatusChange {
                item: ItemReference::Stable(fixture.item_id.clone()),
                status: ItemStatus::InProgress,
                body: None,
            }),
            MindOperationKind::StatusChange,
        ),
        (
            MindRequest::AliasAssignment(AliasAssignment {
                item: ItemReference::Stable(fixture.item_id.clone()),
                alias: ExternalAlias::new("primary-aab"),
            }),
            MindOperationKind::AliasAssignment,
        ),
        (
            MindRequest::Query(Query {
                kind: QueryKind::Ready,
                limit: QueryLimit::new(10),
            }),
            MindOperationKind::Query,
        ),
        (
            MindRequest::AdjudicationRequest(AdjudicationRequest {
                request: sample_adjudication_request(),
                origin: MessageOrigin::External(ConnectionClass::Owner),
                destination: sample_internal_endpoint(ComponentName::Router),
                kind: ChannelMessageKind::MessageSubmission,
                body_summary: TextBody::new("owner request"),
            }),
            MindOperationKind::AdjudicationRequest,
        ),
        (
            MindRequest::ChannelGrant(ChannelGrant {
                source: sample_external_owner_endpoint(),
                destination: sample_internal_endpoint(ComponentName::Router),
                kinds: vec![ChannelMessageKind::MessageSubmission],
                duration: ChannelDuration::Permanent,
            }),
            MindOperationKind::ChannelGrant,
        ),
        (
            MindRequest::ChannelExtend(ChannelExtend {
                channel: sample_channel(),
                duration: ChannelDuration::OneShot,
            }),
            MindOperationKind::ChannelExtend,
        ),
        (
            MindRequest::ChannelRetract(ChannelRetract {
                channel: sample_channel(),
                reason: TextBody::new("operator closed the path"),
            }),
            MindOperationKind::ChannelRetract,
        ),
        (
            MindRequest::AdjudicationDeny(AdjudicationDeny {
                request: sample_adjudication_request(),
                reason: TextBody::new("destination unavailable"),
            }),
            MindOperationKind::AdjudicationDeny,
        ),
        (
            MindRequest::ChannelList(ChannelList { filters: vec![] }),
            MindOperationKind::ChannelList,
        ),
    ];

    for (request, operation) in cases {
        assert_eq!(request.operation_kind(), operation);
    }
}

#[test]
fn mind_graph_request_variants_have_expected_signal_verbs() {
    let graph = MindGraphFixture::new();
    let cases = vec![
        (
            MindRequest::SubmitThought(SubmitThought {
                kind: ThoughtKind::Observation,
                body: graph.observation_body(),
            }),
            SignalVerb::Assert,
        ),
        (
            MindRequest::SubmitRelation(SubmitRelation {
                kind: RelationKind::Implements,
                source: RecordId::new("claim-aab"),
                target: RecordId::new("goal-aab"),
                note: None,
            }),
            SignalVerb::Assert,
        ),
        (
            MindRequest::QueryThoughts(QueryThoughts {
                filter: ThoughtFilter::ByKind(ByThoughtKind {
                    kinds: vec![ThoughtKind::Goal],
                }),
                limit: 10,
            }),
            SignalVerb::Match,
        ),
        (
            MindRequest::QueryRelations(QueryRelations {
                filter: RelationFilter::ByKind(ByRelationKind {
                    kinds: vec![RelationKind::Implements],
                }),
                limit: 10,
            }),
            SignalVerb::Match,
        ),
        (
            MindRequest::SubscribeThoughts(SubscribeThoughts {
                filter: ThoughtFilter::ByAuthor(ByThoughtAuthor {
                    author: ActorName::new("operator"),
                }),
            }),
            SignalVerb::Subscribe,
        ),
        (
            MindRequest::SubscribeRelations(SubscribeRelations {
                filter: RelationFilter::ByTarget(ByRelationTarget {
                    target: RecordId::new("goal-aab"),
                }),
            }),
            SignalVerb::Subscribe,
        ),
    ];

    for (request, verb) in cases {
        assert_eq!(request.signal_verb(), verb);
    }
}

#[test]
fn mind_request_variants_do_not_silently_default_to_assert() {
    let fixture = MemoryFixture::new();
    let cases = vec![
        (
            MindRequest::QueryThoughts(QueryThoughts {
                filter: ThoughtFilter::ByKind(ByThoughtKind {
                    kinds: vec![ThoughtKind::Decision],
                }),
                limit: 12,
            }),
            SignalVerb::Match,
        ),
        (
            MindRequest::SubscribeRelations(SubscribeRelations {
                filter: RelationFilter::ByTarget(ByRelationTarget {
                    target: RecordId::new("decision-aab"),
                }),
            }),
            SignalVerb::Subscribe,
        ),
        (
            MindRequest::RoleRelease(RoleRelease {
                role: RoleName::Operator,
            }),
            SignalVerb::Retract,
        ),
        (
            MindRequest::RoleHandoff(RoleHandoff {
                from: RoleName::Designer,
                to: RoleName::Operator,
                scopes: vec![sample_path_scope()],
                reason: sample_reason(),
            }),
            SignalVerb::Mutate,
        ),
        (
            MindRequest::RoleObservation(RoleObservation),
            SignalVerb::Match,
        ),
        (
            MindRequest::ActivityQuery(ActivityQuery {
                limit: 8,
                filters: vec![ActivityFilter::RoleFilter(RoleName::Operator)],
            }),
            SignalVerb::Match,
        ),
        (
            MindRequest::StatusChange(StatusChange {
                item: ItemReference::Stable(fixture.item_id.clone()),
                status: ItemStatus::InProgress,
                body: None,
            }),
            SignalVerb::Mutate,
        ),
        (
            MindRequest::ChannelRetract(ChannelRetract {
                channel: sample_channel(),
                reason: TextBody::new("retired channel"),
            }),
            SignalVerb::Retract,
        ),
        (
            MindRequest::ChannelList(ChannelList { filters: vec![] }),
            SignalVerb::Match,
        ),
    ];

    for (request, verb) in cases {
        assert_eq!(request.signal_verb(), verb);
        assert_ne!(request.signal_verb(), SignalVerb::Assert);
    }
}

#[test]
fn mind_operation_kind_round_trips_through_nota_text() {
    round_trip_nota(MindOperationKind::ChannelGrant, "ChannelGrant");
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
fn claim_acceptance_reply_round_trips_through_nota_text() {
    round_trip_nota(
        MindReply::ClaimAcceptance(ClaimAcceptance {
            role: RoleName::Designer,
            scopes: vec![sample_task_scope()],
        }),
        "(ClaimAcceptance Designer [(Task primary-f99)])",
    );
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
                reason: ScopeReason::from_text("kameo adoption").expect("scope reason"),
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
fn channel_choreography_replies_round_trip() {
    let replies = vec![
        MindReply::AdjudicationReceipt(AdjudicationReceipt {
            request: sample_adjudication_request(),
        }),
        MindReply::ChannelReceipt(ChannelReceipt {
            channel: sample_channel(),
        }),
        MindReply::AdjudicationDenyReceipt(AdjudicationDenyReceipt {
            request: sample_adjudication_request(),
        }),
        MindReply::ChannelListView(ChannelListView {
            channels: vec![ChannelView {
                channel: sample_channel(),
                source: sample_internal_endpoint(ComponentName::Mind),
                destination: sample_internal_endpoint(ComponentName::Router),
                kinds: vec![
                    ChannelMessageKind::ChannelGrant,
                    ChannelMessageKind::ChannelRetract,
                ],
                duration: ChannelDuration::OneShot,
            }],
        }),
    ];

    for reply in replies {
        let decoded = round_trip_reply(reply.clone());
        assert_eq!(decoded, reply);
    }
}

#[test]
fn explicit_variant_lifts_opening_into_request() {
    let opening = Opening {
        kind: ItemKind::Question,
        priority: ItemPriority::Normal,
        title: Title::new("Choose migration order"),
        body: TextBody::new("Need a decision before implementation."),
    };
    let request = MindRequest::Opening(opening.clone());
    assert_eq!(request, MindRequest::Opening(opening));
}

#[test]
fn explicit_variant_lifts_view_into_reply() {
    let view = MemoryFixture::new().view();
    let reply = MindReply::View(view.clone());
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
fn wire_path_nota_decode_uses_boundary_validation() {
    let mut decoder = Decoder::new("\"relative/path\"");
    let error = WirePath::decode(&mut decoder).expect_err("relative path must fail validation");
    match error {
        NotaError::Validation { type_name, message } => {
            assert_eq!(type_name, "WirePath");
            assert!(message.contains("absolute"), "message was: {message}");
        }
        other => panic!("expected validation error, got {other:?}"),
    }
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
