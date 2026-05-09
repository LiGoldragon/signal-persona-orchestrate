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
    WirePath::new("/git/github.com/LiGoldragon/signal-persona-orchestrate/src/lib.rs")
}

fn sample_task() -> TaskToken {
    TaskToken::new("primary-f99")
}

fn sample_reason() -> ScopeReason {
    ScopeReason::new("design-cascade per /93")
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
        reason: ScopeReason::new("router migration handoff"),
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
        reason: ScopeReason::new("audit signal-persona-system integration"),
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
        filters: vec![ActivityFilter::PathPrefix(WirePath::new(
            "/git/github.com/LiGoldragon/persona-router",
        ))],
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
            held_reason: ScopeReason::new("Persona-prefix sweep"),
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
            held_reason: ScopeReason::new("audit pass"),
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
                reason: ScopeReason::new("rescope per /91 §3.1"),
                stamped_at: TimestampNanos::new(1_730_000_000_000_000_000),
            },
            Activity {
                role: RoleName::Operator,
                scope: sample_task_scope(),
                reason: ScopeReason::new("ractor adoption"),
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
