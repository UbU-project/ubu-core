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

#[derive(Debug, Default, Clone, Copy)]
pub struct Legitimizer;

impl Legitimizer {
    pub fn adjudicate_export_projection(
        context: ExportProjectionContext<'_>,
    ) -> LegitimizerDecision {
        let adjudication = adjudicate(context.operation, context.effective_policy);
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
) -> ExportAdjudication {
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

    ExportAdjudication {
        legitimization: Legitimization::Rejected,
        member_evaluated: PolicyMember::NoExternalExport,
        reasons: vec![format!(
            "Resolved Compartment policy does not explicitly permit external export for projection operation {}; export is rejected by default.",
            operation.operation_id
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id_registry::ObjectType;
    use crate::projection::ProjectionOperationKind;
    use crate::{SourceRef, UbuId};

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

    fn decide(effective_policy: Option<&PolicySummary>) -> LegitimizerDecision {
        let operation = operation();
        let compartment_ref = object_ref(ObjectType::Compartment);
        let actor_identity_ref = object_ref(ObjectType::Identity);
        let provenance = provenance();

        Legitimizer::adjudicate_export_projection(ExportProjectionContext {
            operation: &operation,
            effective_policy,
            compartment_ref: &compartment_ref,
            actor_identity_ref: &actor_identity_ref,
            authority_source: AuthoritySource::Policy,
            effective_time: timestamp(),
            provenance: &provenance,
        })
    }

    #[test]
    fn accepts_export_when_policy_explicitly_permits_it() {
        let effective_policy = policy(Legitimization::Accepted, Some(false), Some(false));

        let decision = decide(Some(&effective_policy));

        assert_eq!(decision.legitimization, Legitimization::Accepted);
        assert!(!decision.adjudication_reasons.is_empty());
        assert_eq!(
            decision.log_payload.adjudication_result,
            Legitimization::Accepted
        );
        assert_eq!(
            decision.log_payload.member_evaluated,
            PolicyMember::NoExternalExport
        );
        assert_eq!(
            decision.log_payload.compartment_ref.object_type,
            ObjectType::Compartment
        );
        assert_eq!(
            decision.log_payload.actor_identity_ref.object_type,
            ObjectType::Identity
        );
        assert_eq!(
            decision.log_payload.authority_source,
            AuthoritySource::Policy
        );
        assert_eq!(decision.log_payload.effective_time, timestamp());
        assert_eq!(
            decision.log_payload.provenance.authority_source,
            AuthoritySource::Policy
        );
        assert!(!decision.log_payload.reason.is_empty());
    }

    #[test]
    fn rejects_export_when_no_external_export_is_set() {
        let effective_policy = policy(Legitimization::Accepted, Some(false), Some(true));

        let decision = decide(Some(&effective_policy));

        assert_eq!(decision.legitimization, Legitimization::Rejected);
        assert!(decision
            .adjudication_reasons
            .iter()
            .any(|reason| reason.contains("no_external_export")));
        assert_eq!(
            decision.log_payload.adjudication_result,
            Legitimization::Rejected
        );
        assert_eq!(
            decision.log_payload.member_evaluated,
            PolicyMember::NoExternalExport
        );
        assert!(!decision.log_payload.reason.is_empty());
    }

    #[test]
    fn rejects_export_when_local_only_is_set() {
        let effective_policy = policy(Legitimization::Accepted, Some(true), Some(false));

        let decision = decide(Some(&effective_policy));

        assert_eq!(decision.legitimization, Legitimization::Rejected);
        assert!(decision
            .adjudication_reasons
            .iter()
            .any(|reason| reason.contains("local_only")));
        assert_eq!(
            decision.log_payload.adjudication_result,
            Legitimization::Rejected
        );
        assert_eq!(
            decision.log_payload.member_evaluated,
            PolicyMember::LocalOnly
        );
        assert!(!decision.log_payload.reason.is_empty());
    }

    #[test]
    fn rejects_export_when_policy_is_unresolved() {
        let decision = decide(None);

        assert_eq!(decision.legitimization, Legitimization::Rejected);
        assert!(decision
            .adjudication_reasons
            .iter()
            .any(|reason| reason.contains("could not be resolved")));
        assert_eq!(
            decision.log_payload.adjudication_result,
            Legitimization::Rejected
        );
        assert_eq!(
            decision.log_payload.member_evaluated,
            PolicyMember::NoExternalExport
        );
        assert!(!decision.log_payload.reason.is_empty());
    }
}
