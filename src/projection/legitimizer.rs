use crate::authority::AuthoritySource;
use crate::core::{CompartmentBoundaryDecidedPayload, PolicyMember};
use crate::object_ref::ObjectRef;
use crate::policy_summary::{Legitimization, PolicySummary};
use crate::projection::operation::ProjectionOperation;
use crate::provenance::Provenance;
use crate::time::UbuTimestamp;

#[derive(Debug, Clone, Copy)]
pub struct ExportProjectionContext<'a> {
    pub operation: &'a ProjectionOperation,
    pub effective_policy: Option<&'a PolicySummary>,
    pub compartment_ref: &'a ObjectRef,
    pub actor_identity_ref: &'a ObjectRef,
    pub authority_source: AuthoritySource,
    pub effective_time: UbuTimestamp,
    pub provenance: &'a Provenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegitimizerDecision {
    pub legitimization: Legitimization,
    pub adjudication_reasons: Vec<String>,
    pub log_payload: CompartmentBoundaryDecidedPayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportPermit {
    operation_id: String,
    authority_source: AuthoritySource,
}

impl ExportPermit {
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    pub fn authority_source(&self) -> AuthoritySource {
        self.authority_source
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportGateDecision {
    pub decision: LegitimizerDecision,
    permit: Option<ExportPermit>,
}

impl ExportGateDecision {
    pub fn permit(&self) -> Option<&ExportPermit> {
        self.permit.as_ref()
    }

    pub fn into_decision(self) -> LegitimizerDecision {
        self.decision
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Legitimizer;

impl Legitimizer {
    pub fn gate_export_projection(context: ExportProjectionContext<'_>) -> ExportGateDecision {
        let operation_id = context.operation.operation_id.clone();
        let decision = Self::adjudicate_export_projection(context);
        let permit =
            (decision.legitimization == Legitimization::Accepted).then_some(ExportPermit {
                operation_id,
                authority_source: AuthoritySource::AutomationWorker,
            });

        ExportGateDecision { decision, permit }
    }

    pub fn adjudicate_export_projection(
        context: ExportProjectionContext<'_>,
    ) -> LegitimizerDecision {
        let adjudication = adjudicate(
            context.operation,
            context.effective_policy,
            context.authority_source,
        );
        let reason = adjudication.reasons.join(" ");

        let log_payload = CompartmentBoundaryDecidedPayload {
            compartment_ref: context.compartment_ref.clone(),
            member_evaluated: adjudication.member_evaluated,
            adjudication_result: adjudication.legitimization,
            actor_identity_ref: context.actor_identity_ref.clone(),
            authority_source: context.authority_source,
            reason,
            effective_time: context.effective_time,
            provenance: context.provenance.clone(),
        };

        LegitimizerDecision {
            legitimization: adjudication.legitimization,
            adjudication_reasons: adjudication.reasons,
            log_payload,
        }
    }
}

struct ExportAdjudication {
    legitimization: Legitimization,
    member_evaluated: PolicyMember,
    reasons: Vec<String>,
}

fn adjudicate(
    operation: &ProjectionOperation,
    effective_policy: Option<&PolicySummary>,
    authority_source: AuthoritySource,
) -> ExportAdjudication {
    if authority_source != AuthoritySource::AutomationWorker {
        let reason = match authority_source {
            AuthoritySource::User | AuthoritySource::UserOverride => format!(
                "Projection export operation {} used user-equivalent authority {}; export is rejected by default. Export-class operations require automation_worker authority.",
                operation.operation_id,
                authority_source_wire(authority_source)
            ),
            _ => format!(
                "Projection export operation {} used authority {}; export is rejected by default. Export-class operations require automation_worker authority.",
                operation.operation_id,
                authority_source_wire(authority_source)
            ),
        };

        return ExportAdjudication {
            legitimization: Legitimization::Rejected,
            member_evaluated: PolicyMember::NoExternalExport,
            reasons: vec![reason],
        };
    }

    let Some(policy) = effective_policy else {
        return ExportAdjudication {
            legitimization: Legitimization::Rejected,
            member_evaluated: PolicyMember::NoExternalExport,
            reasons: vec![format!(
                "Effective Compartment policy could not be resolved for projection operation {}; export is rejected by default.",
                operation.operation_id
            )],
        };
    };

    let mut rejecting_members = Vec::new();
    let mut reasons = Vec::new();

    if policy.local_only == Some(true) {
        rejecting_members.push(PolicyMember::LocalOnly);
        reasons.push(format!(
            "Compartment policy local_only forbids external export for projection operation {}.",
            operation.operation_id
        ));
    }

    if policy.no_external_export == Some(true) {
        rejecting_members.push(PolicyMember::NoExternalExport);
        reasons.push(format!(
            "Compartment policy no_external_export forbids external export for projection operation {}.",
            operation.operation_id
        ));
    }

    if let Some(member_evaluated) = rejecting_members.first().copied() {
        return ExportAdjudication {
            legitimization: Legitimization::Rejected,
            member_evaluated,
            reasons,
        };
    }

    if policy.legitimization == Legitimization::Accepted
        && policy.local_only == Some(false)
        && policy.no_external_export == Some(false)
    {
        return ExportAdjudication {
            legitimization: Legitimization::Accepted,
            member_evaluated: PolicyMember::NoExternalExport,
            reasons: vec![format!(
                "Resolved Compartment policy explicitly permits external export for projection operation {}.",
                operation.operation_id
            )],
        };
    }

    if policy.legitimization != Legitimization::Accepted {
        return ExportAdjudication {
            legitimization: policy.legitimization,
            member_evaluated: PolicyMember::NoExternalExport,
            reasons: vec![format!(
                "Resolved Compartment policy returned {} for projection operation {}; export is denied by default because the legitimization is not accepted.",
                legitimization_wire(policy.legitimization),
                operation.operation_id
            )],
        };
    }

    ExportAdjudication {
        legitimization: Legitimization::Rejected,
        member_evaluated: PolicyMember::NoExternalExport,
        reasons: vec![format!(
            "Resolved Compartment policy does not explicitly permit external export for projection operation {}; export is rejected by default.",
            operation.operation_id
        )],
    }
}

fn authority_source_wire(authority_source: AuthoritySource) -> &'static str {
    match authority_source {
        AuthoritySource::User => "user",
        AuthoritySource::UserOverride => "user_override",
        AuthoritySource::Delegated => "delegated",
        AuthoritySource::AutomationWorker => "automation_worker",
        AuthoritySource::Policy => "policy",
        AuthoritySource::System => "system",
    }
}

fn legitimization_wire(legitimization: Legitimization) -> &'static str {
    match legitimization {
        Legitimization::Accepted => "accepted",
        Legitimization::NeedsReview => "needs_review",
        Legitimization::Rejected => "rejected",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id_registry::ObjectType;
    use crate::projection::ProjectionOperationKind;
    use crate::{SourceRef, UbuId};
    use serde_json::json;

    struct PolicyCase {
        name: &'static str,
        effective_policy: Option<PolicySummary>,
        worker_legitimization: Legitimization,
    }

    fn timestamp() -> UbuTimestamp {
        UbuTimestamp::parse("2026-06-10T14:30:00Z").expect("valid timestamp")
    }

    fn operation() -> ProjectionOperation {
        ProjectionOperation {
            operation_id: "op-label-1".to_owned(),
            kind: ProjectionOperationKind::Label,
            target: SourceRef {
                source_kind: "github_issue".to_owned(),
                source_id: "UbU-project/ubu-core#123".to_owned(),
                url: None,
            },
            summary: "Apply managed GitHub label".to_owned(),
            payload: None,
        }
    }

    fn operation_with_sensitive_boundary_identity() -> ProjectionOperation {
        ProjectionOperation {
            operation_id: "op-redaction-identity".to_owned(),
            kind: ProjectionOperationKind::Label,
            target: SourceRef {
                source_kind: "github_issue".to_owned(),
                source_id: "UbU-project/ubu-core#456".to_owned(),
                url: None,
            },
            summary: "Apply Project Cerulean / S2-secret-label".to_owned(),
            payload: Some(json!({
                "compartment_name": "Project Cerulean",
                "labels": ["S2-secret-label"],
            })),
        }
    }

    fn policy(
        legitimization: Legitimization,
        local_only: Option<bool>,
        no_external_export: Option<bool>,
    ) -> PolicySummary {
        PolicySummary {
            legitimization,
            adjudication_reasons: Vec::new(),
            local_only,
            no_cloud_llm: None,
            no_external_export,
            checked_at: timestamp(),
        }
    }

    fn object_ref(object_type: ObjectType) -> ObjectRef {
        ObjectRef {
            id: UbuId::new(object_type),
            object_type,
        }
    }

    fn provenance() -> Provenance {
        Provenance {
            created_at: timestamp(),
            created_by: Some("legitimizer-test".to_owned()),
            authority_source: AuthoritySource::Policy,
            source: None,
            source_refs: None,
        }
    }

    fn gate(
        operation: &ProjectionOperation,
        effective_policy: Option<&PolicySummary>,
        authority_source: AuthoritySource,
    ) -> ExportGateDecision {
        let compartment_ref = object_ref(ObjectType::Compartment);
        let actor_identity_ref = object_ref(ObjectType::Identity);
        let provenance = provenance();

        Legitimizer::gate_export_projection(ExportProjectionContext {
            operation,
            effective_policy,
            compartment_ref: &compartment_ref,
            actor_identity_ref: &actor_identity_ref,
            authority_source,
            effective_time: timestamp(),
            provenance: &provenance,
        })
    }

    fn policy_cases() -> Vec<PolicyCase> {
        vec![
            PolicyCase {
                name: "export_permitted",
                effective_policy: Some(policy(Legitimization::Accepted, Some(false), Some(false))),
                worker_legitimization: Legitimization::Accepted,
            },
            PolicyCase {
                name: "no_external_export",
                effective_policy: Some(policy(Legitimization::Accepted, Some(false), Some(true))),
                worker_legitimization: Legitimization::Rejected,
            },
            PolicyCase {
                name: "local_only",
                effective_policy: Some(policy(Legitimization::Accepted, Some(true), Some(false))),
                worker_legitimization: Legitimization::Rejected,
            },
            PolicyCase {
                name: "unresolved_policy",
                effective_policy: None,
                worker_legitimization: Legitimization::Rejected,
            },
            PolicyCase {
                name: "needs_review",
                effective_policy: Some(policy(
                    Legitimization::NeedsReview,
                    Some(false),
                    Some(false),
                )),
                worker_legitimization: Legitimization::NeedsReview,
            },
        ]
    }

    fn authority_sources() -> [AuthoritySource; 6] {
        [
            AuthoritySource::User,
            AuthoritySource::UserOverride,
            AuthoritySource::Delegated,
            AuthoritySource::AutomationWorker,
            AuthoritySource::Policy,
            AuthoritySource::System,
        ]
    }

    #[test]
    fn export_gate_denies_by_default_and_only_permits_worker_accepted_adjudication() {
        let operation = operation();

        for policy_case in policy_cases() {
            for authority_source in authority_sources() {
                let gate_decision = gate(
                    &operation,
                    policy_case.effective_policy.as_ref(),
                    authority_source,
                );
                let decision = &gate_decision.decision;
                let expected_legitimization =
                    if authority_source == AuthoritySource::AutomationWorker {
                        policy_case.worker_legitimization
                    } else {
                        Legitimization::Rejected
                    };
                let should_permit = expected_legitimization == Legitimization::Accepted
                    && authority_source == AuthoritySource::AutomationWorker;

                assert_eq!(
                    decision.legitimization, expected_legitimization,
                    "{} with {:?}",
                    policy_case.name, authority_source
                );
                assert_eq!(
                    decision.log_payload.adjudication_result, expected_legitimization,
                    "{} with {:?}",
                    policy_case.name, authority_source
                );
                assert_eq!(
                    gate_decision.permit().is_some(),
                    should_permit,
                    "{} with {:?}",
                    policy_case.name,
                    authority_source
                );
                assert!(!decision.adjudication_reasons.is_empty());
                assert!(!decision.log_payload.reason.is_empty());
                assert_eq!(
                    decision.log_payload.compartment_ref.object_type,
                    ObjectType::Compartment
                );
                assert_eq!(
                    decision.log_payload.actor_identity_ref.object_type,
                    ObjectType::Identity
                );
                assert_eq!(decision.log_payload.authority_source, authority_source);
                assert_eq!(decision.log_payload.effective_time, timestamp());

                if let Some(permit) = gate_decision.permit() {
                    assert_eq!(permit.operation_id(), operation.operation_id);
                    assert_eq!(permit.authority_source(), AuthoritySource::AutomationWorker);
                }

                if matches!(
                    authority_source,
                    AuthoritySource::User | AuthoritySource::UserOverride
                ) {
                    assert!(decision
                        .adjudication_reasons
                        .iter()
                        .any(|reason| reason.contains("user-equivalent")));
                }

                if authority_source != AuthoritySource::AutomationWorker {
                    assert!(decision
                        .adjudication_reasons
                        .iter()
                        .any(|reason| reason.contains("automation_worker")));
                }
            }
        }
    }

    #[test]
    fn denied_export_path_does_not_leak_compartment_names_or_labels() {
        let operation = operation_with_sensitive_boundary_identity();
        let effective_policy = policy(Legitimization::Accepted, Some(true), Some(false));
        let gate_decision = gate(
            &operation,
            Some(&effective_policy),
            AuthoritySource::AutomationWorker,
        );

        assert_eq!(
            gate_decision.decision.legitimization,
            Legitimization::Rejected
        );
        assert!(gate_decision.permit().is_none());

        let denial_reasons =
            serde_json::to_string(&gate_decision.decision.adjudication_reasons).unwrap();
        let log_payload = serde_json::to_string(&gate_decision.decision.log_payload).unwrap();

        for denied_boundary_identity in ["Project Cerulean", "S2-secret-label"] {
            assert!(
                !denial_reasons.contains(denied_boundary_identity),
                "denial reasons leaked {denied_boundary_identity}"
            );
            assert!(
                !log_payload.contains(denied_boundary_identity),
                "log payload leaked {denied_boundary_identity}"
            );
        }

        // TODO(C7-export-boundary-redaction): extend this invariant to all
        // serializers once cross-cutting serializer redaction exists.
    }
}
