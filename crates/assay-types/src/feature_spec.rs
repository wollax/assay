//! IEEE 830/29148-inspired feature specification types.
//!
//! A `FeatureSpec` represents a per-feature specification loaded from
//! `.assay/specs/<feature>/spec.toml`. It captures requirements, constraints,
//! users, quality attributes, assumptions, dependencies, and risks in a
//! structured, machine-readable format.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Status of a feature spec or individual requirement through its lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SpecStatus {
    #[default]
    Draft,
    Proposed,
    Planned,
    InProgress,
    Verified,
    Deprecated,
}

impl std::fmt::Display for SpecStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Proposed => write!(f, "proposed"),
            Self::Planned => write!(f, "planned"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Verified => write!(f, "verified"),
            Self::Deprecated => write!(f, "deprecated"),
        }
    }
}

/// RFC 2119 obligation level for a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Obligation {
    #[default]
    Shall,
    Should,
    May,
}

impl std::fmt::Display for Obligation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shall => write!(f, "shall"),
            Self::Should => write!(f, "should"),
            Self::May => write!(f, "may"),
        }
    }
}

/// MoSCoW priority for a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    #[default]
    Must,
    Should,
    Could,
    Wont,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Must => write!(f, "must"),
            Self::Should => write!(f, "should"),
            Self::Could => write!(f, "could"),
            Self::Wont => write!(f, "wont"),
        }
    }
}

/// Verification method for a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum VerificationMethod {
    #[default]
    Test,
    Analysis,
    Inspection,
    Demonstration,
}

impl std::fmt::Display for VerificationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Test => write!(f, "test"),
            Self::Analysis => write!(f, "analysis"),
            Self::Inspection => write!(f, "inspection"),
            Self::Demonstration => write!(f, "demonstration"),
        }
    }
}

/// Acceptance criterion type for a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum AcceptanceCriterionType {
    Gherkin,
    Ears,
    #[default]
    Plain,
}

impl std::fmt::Display for AcceptanceCriterionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gherkin => write!(f, "gherkin"),
            Self::Ears => write!(f, "ears"),
            Self::Plain => write!(f, "plain"),
        }
    }
}

/// A single acceptance criterion attached to a requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCriterion {
    /// The criterion statement (Gherkin, EARS, or plain text).
    pub criterion: String,

    /// Format of this acceptance criterion.
    #[serde(default, rename = "type")]
    pub criterion_type: AcceptanceCriterionType,
}

/// A structured requirement within a feature spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    /// Unique requirement ID in `REQ-[AREA]-[NNN]` format.
    pub id: String,

    /// Short human-readable title.
    pub title: String,

    /// The normative requirement statement.
    pub statement: String,

    /// Why this requirement exists.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,

    /// RFC 2119 obligation level.
    #[serde(default)]
    pub obligation: Obligation,

    /// MoSCoW priority.
    #[serde(default)]
    pub priority: Priority,

    /// How this requirement is verified.
    #[serde(default)]
    pub verification: VerificationMethod,

    /// Lifecycle status of this requirement.
    #[serde(default)]
    pub status: SpecStatus,

    /// Acceptance criteria for this requirement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
}

/// Overview section of a feature spec.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeatureOverview {
    /// How this feature fits into the larger system.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub perspective: String,

    /// High-level functions this feature provides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<String>,

    /// Summary description of the feature.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// Constraints that bound the feature implementation.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Constraints {
    /// Technology constraints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub technology: Vec<String>,

    /// Regulatory/compliance constraints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regulatory: Vec<String>,

    /// Performance constraints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub performance: Vec<String>,
}

/// A user class that interacts with this feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct UserClass {
    /// Name of this user class.
    pub name: String,

    /// Expertise level of this user class.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expertise: String,

    /// Usage frequency.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub frequency: String,

    /// Goals this user class has for this feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub goals: Vec<String>,
}

/// A quality attribute (non-functional requirement).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualityAttribute {
    /// Description of this quality requirement.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// Quantitative target (e.g., response time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time_ms: Option<u64>,

    /// Security controls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<String>,
}

/// Quality attributes section keyed by category.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub struct QualityAttributes {
    /// Performance quality attributes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performance: Option<QualityAttribute>,

    /// Security quality attributes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<QualityAttribute>,

    /// Reliability quality attributes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reliability: Option<QualityAttribute>,

    /// Usability quality attributes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usability: Option<QualityAttribute>,
}

/// An assumption that, if false, affects the feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Assumption {
    /// Description of the assumption.
    pub description: String,

    /// Impact if the assumption proves false.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub impact_if_false: String,
}

/// An external dependency of this feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    /// Name of the dependency.
    pub name: String,

    /// Type of dependency (e.g., "external_service", "library", "internal_module").
    #[serde(default, skip_serializing_if = "String::is_empty", rename = "type")]
    pub dependency_type: String,

    /// Description of this dependency.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

/// Impact level for risk assessment.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ImpactLevel {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// Likelihood level for risk assessment.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LikelihoodLevel {
    Low,
    #[default]
    Medium,
    High,
}

impl std::fmt::Display for LikelihoodLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

/// A risk associated with this feature.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Risk {
    /// Description of the risk.
    pub description: String,

    /// Impact if the risk materializes.
    #[serde(default)]
    pub impact: ImpactLevel,

    /// Likelihood of the risk materializing.
    #[serde(default)]
    pub likelihood: LikelihoodLevel,

    /// Planned mitigation strategy.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mitigation: String,
}

/// Verification strategy for the feature as a whole.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VerificationStrategy {
    /// Overall verification strategy description.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub strategy: String,

    /// Target environments for verification.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub environments: Vec<String>,
}

/// A feature specification following IEEE 830/29148 principles.
///
/// Loaded from `.assay/specs/<feature>/spec.toml`. Captures the full
/// requirements context for a feature: what to build, who it's for,
/// constraints, quality attributes, risks, and verification strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FeatureSpec {
    /// Feature name (must match the directory name / gates.toml name).
    pub name: String,

    /// Lifecycle status of the overall feature.
    #[serde(default)]
    pub status: SpecStatus,

    /// Specification version.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,

    /// Feature overview: perspective, functions, description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overview: Option<FeatureOverview>,

    /// Implementation constraints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Constraints>,

    /// User classes that interact with this feature.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub users: Vec<UserClass>,

    /// Structured requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<Requirement>,

    /// Quality attributes (non-functional requirements).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<QualityAttributes>,

    /// Assumptions about the operating environment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<Assumption>,

    /// External dependencies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    /// Identified risks and mitigations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub risks: Vec<Risk>,

    /// Verification strategy for the feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<VerificationStrategy>,
}

inventory::submit! {
    crate::schema_registry::SchemaEntry {
        name: "feature-spec",
        generate: || schemars::schema_for!(FeatureSpec),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_feature_spec() -> FeatureSpec {
        FeatureSpec {
            name: "auth-flow".to_string(),
            status: SpecStatus::Draft,
            version: String::new(),
            overview: None,
            constraints: None,
            users: vec![],
            requirements: vec![],
            quality: None,
            assumptions: vec![],
            dependencies: vec![],
            risks: vec![],
            verification: None,
        }
    }

    #[test]
    fn minimal_feature_spec_toml_roundtrip() {
        let spec = minimal_feature_spec();
        let toml_str = toml::to_string(&spec).expect("serialize to TOML");
        let roundtripped: FeatureSpec = toml::from_str(&toml_str).expect("deserialize from TOML");
        assert_eq!(spec, roundtripped);
    }

    #[test]
    fn full_feature_spec_toml_roundtrip() {
        let toml_str = r#"
name = "auth-flow"
status = "draft"
version = "0.1"

[overview]
perspective = "Part of the identity management system"
functions = ["Login with credentials", "OAuth2 social login"]
description = "User authentication and authorization flow"

[constraints]
technology = ["Must use OAuth 2.0", "JWT for session tokens"]
regulatory = ["GDPR Article 7 - consent"]
performance = ["Login < 2 seconds at p99"]

[[users]]
name = "End User"
expertise = "non-technical"
frequency = "daily"
goals = ["Quick frictionless login"]

[[requirements]]
id = "REQ-FUNC-001"
title = "Login with credentials"
statement = "The system SHALL authenticate users with email and password"
rationale = "Users need secure, standard authentication"
obligation = "shall"
priority = "must"
verification = "test"
status = "draft"

  [[requirements.acceptance_criteria]]
  criterion = "Given valid credentials, When the user submits the login form, Then they are redirected to the dashboard within 2 seconds"
  type = "gherkin"

[quality]
  [quality.performance]
  response_time_ms = 2000
  description = "Login endpoint must respond within 2s at 100 RPS"

  [quality.security]
  controls = ["Rate limiting", "Account lockout", "CSRF protection"]

[[assumptions]]
description = "OAuth2 provider will be available with 99.9% uptime"
impact_if_false = "Must implement custom OAuth server"

[[dependencies]]
name = "auth0"
type = "external_service"
description = "OAuth2/OIDC identity provider"

[[risks]]
description = "OAuth provider rate limits during peak traffic"
impact = "high"
likelihood = "medium"
mitigation = "Token caching and local session management"

[verification]
strategy = "Integration tests with mock OAuth provider"
environments = ["CI", "staging"]
"#;

        let spec: FeatureSpec = toml::from_str(toml_str).expect("parse full feature spec");

        assert_eq!(spec.name, "auth-flow");
        assert_eq!(spec.status, SpecStatus::Draft);
        assert_eq!(spec.version, "0.1");

        let overview = spec.overview.as_ref().unwrap();
        assert_eq!(overview.functions.len(), 2);

        let constraints = spec.constraints.as_ref().unwrap();
        assert_eq!(constraints.technology.len(), 2);

        assert_eq!(spec.users.len(), 1);
        assert_eq!(spec.users[0].name, "End User");

        assert_eq!(spec.requirements.len(), 1);
        let req = &spec.requirements[0];
        assert_eq!(req.id, "REQ-FUNC-001");
        assert_eq!(req.obligation, Obligation::Shall);
        assert_eq!(req.priority, Priority::Must);
        assert_eq!(req.acceptance_criteria.len(), 1);
        assert_eq!(
            req.acceptance_criteria[0].criterion_type,
            AcceptanceCriterionType::Gherkin
        );

        let quality = spec.quality.as_ref().unwrap();
        assert_eq!(
            quality.performance.as_ref().unwrap().response_time_ms,
            Some(2000)
        );
        assert_eq!(quality.security.as_ref().unwrap().controls.len(), 3);

        assert_eq!(spec.assumptions.len(), 1);
        assert_eq!(spec.dependencies.len(), 1);
        assert_eq!(spec.risks.len(), 1);
        assert_eq!(spec.risks[0].impact, ImpactLevel::High);
        assert_eq!(spec.risks[0].likelihood, LikelihoodLevel::Medium);

        let verification = spec.verification.as_ref().unwrap();
        assert_eq!(verification.environments.len(), 2);

        // Roundtrip
        let re_serialized = toml::to_string(&spec).expect("re-serialize");
        let roundtripped: FeatureSpec =
            toml::from_str(&re_serialized).expect("roundtrip deserialize");
        assert_eq!(spec, roundtripped);
    }

    #[test]
    fn feature_spec_rejects_unknown_fields() {
        let toml_str = r#"
name = "test"
unknown_field = "oops"
"#;
        let err = toml::from_str::<FeatureSpec>(toml_str).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown field"),
            "should reject unknown field, got: {msg}"
        );
    }

    #[test]
    fn spec_status_kebab_case_serde() {
        let toml_str = r#"
name = "test"
status = "in-progress"
"#;
        let spec: FeatureSpec = toml::from_str(toml_str).expect("parse in-progress status");
        assert_eq!(spec.status, SpecStatus::InProgress);
    }

    #[test]
    fn obligation_kebab_case_serde() {
        // These are all lowercase single-word, so kebab-case doesn't change them
        let json = serde_json::to_string(&Obligation::Shall).unwrap();
        assert_eq!(json, r#""shall""#);
    }

    #[test]
    fn priority_wont_serde() {
        let json = serde_json::to_string(&Priority::Wont).unwrap();
        assert_eq!(json, r#""wont""#);
    }
}
