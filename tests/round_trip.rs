//! Architectural-truth round-trip tests for the
//! `signal-persona-orchestrate` channel.
//!
//! Per `~/primary/skills/architectural-truth-tests.md`,
//! each variant of both enums has a witness test that
//! proves the macro-emitted type round-trips through a
//! length-prefixed Frame.

use signal_core::{FrameBody, Reply, Request, SemaVerb};
use signal_persona_orchestrate::{
    Activity, ActivityAcknowledgment, ActivityFilter, ActivityList, ActivityQuery,
    ActivitySubmission, ClaimAcceptance, ClaimEntry, ClaimRejection, Frame, HandoffAcceptance,
    HandoffRejection, HandoffRejectionReason, OrchestrateReply, OrchestrateRequest,
    ReleaseAcknowledgment, RoleClaim, RoleHandoff, RoleName, RoleObservation, RoleRelease,
    RoleSnapshot, RoleStatus, ScopeConflict, ScopeReason, ScopeReference, TaskToken,
    TimestampNanos, WirePath,
};

// ─── Helpers ──────────────────────────────────────────────

fn round_trip_request(request: OrchestrateRequest) -> OrchestrateRequest {
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

fn round_trip_reply(reply: OrchestrateReply) -> OrchestrateReply {
    let frame = Frame::new(FrameBody::Reply(Reply::operation(reply)));
    let bytes = frame.encode_length_prefixed().expect("encode");
    let decoded = Frame::decode_length_prefixed(&bytes).expect("decode");
    match decoded.into_body() {
        FrameBody::Reply(Reply::Operation(reply)) => reply,
        other => panic!("expected reply operation, got {other:?}"),
    }
}

fn sample_path() -> WirePath {
    WirePath::from_absolute_path(
        "/git/github.com/LiGoldragon/signal-persona-orchestrate/src/lib.rs",
    )
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

// ─── Request variants ─────────────────────────────────────

#[test]
fn role_claim_with_paths_round_trips() {
    let request = OrchestrateRequest::RoleClaim(RoleClaim {
        role: RoleName::Designer,
        scopes: vec![sample_path_scope(), sample_task_scope()],
        reason: sample_reason(),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn role_release_round_trips() {
    let request = OrchestrateRequest::RoleRelease(RoleRelease {
        role: RoleName::Operator,
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn role_handoff_round_trips() {
    let request = OrchestrateRequest::RoleHandoff(RoleHandoff {
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
    let request = OrchestrateRequest::RoleObservation(RoleObservation);
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_submission_round_trips() {
    let request = OrchestrateRequest::ActivitySubmission(ActivitySubmission {
        role: RoleName::Assistant,
        scope: sample_path_scope(),
        reason: ScopeReason::from_text("audit signal-persona-system integration")
            .expect("scope reason"),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_unfiltered_round_trips() {
    let request = OrchestrateRequest::ActivityQuery(ActivityQuery {
        limit: 25,
        filters: vec![],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_with_role_filter_round_trips() {
    let request = OrchestrateRequest::ActivityQuery(ActivityQuery {
        limit: 50,
        filters: vec![ActivityFilter::RoleFilter(RoleName::Operator)],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn activity_query_with_path_prefix_round_trips() {
    let request = OrchestrateRequest::ActivityQuery(ActivityQuery {
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
    let request = OrchestrateRequest::ActivityQuery(ActivityQuery {
        limit: 100,
        filters: vec![ActivityFilter::TaskToken(sample_task())],
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

// ─── Reply variants ───────────────────────────────────────

#[test]
fn claim_acceptance_round_trips() {
    let reply = OrchestrateReply::ClaimAcceptance(ClaimAcceptance {
        role: RoleName::Designer,
        scopes: vec![sample_path_scope()],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn claim_rejection_round_trips() {
    let reply = OrchestrateReply::ClaimRejection(ClaimRejection {
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
    let reply = OrchestrateReply::ReleaseAcknowledgment(ReleaseAcknowledgment {
        role: RoleName::Designer,
        released_scopes: vec![sample_path_scope(), sample_task_scope()],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn handoff_acceptance_round_trips() {
    let reply = OrchestrateReply::HandoffAcceptance(HandoffAcceptance {
        from: RoleName::Designer,
        to: RoleName::Operator,
        scopes: vec![sample_path_scope()],
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn handoff_rejection_source_does_not_hold_round_trips() {
    let reply = OrchestrateReply::HandoffRejection(HandoffRejection {
        from: RoleName::Designer,
        to: RoleName::Operator,
        reason: HandoffRejectionReason::SourceRoleDoesNotHold,
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn handoff_rejection_target_conflict_round_trips() {
    let reply = OrchestrateReply::HandoffRejection(HandoffRejection {
        from: RoleName::Designer,
        to: RoleName::Operator,
        reason: HandoffRejectionReason::TargetRoleConflict(vec![ScopeConflict {
            scope: sample_path_scope(),
            held_by: RoleName::Assistant,
            held_reason: ScopeReason::from_text("audit pass").expect("scope reason"),
        }]),
    });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn role_snapshot_round_trips() {
    let reply = OrchestrateReply::RoleSnapshot(RoleSnapshot {
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
    let reply = OrchestrateReply::ActivityAcknowledgment(ActivityAcknowledgment { slot: 42 });
    let decoded = round_trip_reply(reply.clone());
    assert_eq!(decoded, reply);
}

#[test]
fn activity_list_round_trips() {
    let reply = OrchestrateReply::ActivityList(ActivityList {
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

// ─── Scope-reference variants ─────────────────────────────

#[test]
fn path_scope_round_trips() {
    let request = OrchestrateRequest::RoleClaim(RoleClaim {
        role: RoleName::Designer,
        scopes: vec![ScopeReference::Path(sample_path())],
        reason: sample_reason(),
    });
    let decoded = round_trip_request(request.clone());
    assert_eq!(decoded, request);
}

#[test]
fn task_scope_round_trips() {
    let request = OrchestrateRequest::RoleClaim(RoleClaim {
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
