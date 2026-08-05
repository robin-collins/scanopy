use std::{collections::HashSet, fmt, net::IpAddr, str::FromStr};

use chrono::{DateTime, Duration, Utc};
use ipnetwork::IpNetwork;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

pub const MAX_DOMAINS_PER_COLLECTION: usize = 64;
pub const MAX_ENTITIES_PER_COLLECTION: usize = 3_000;
pub const MAX_ISSUES_PER_COLLECTION: usize = 100;
pub const MAX_ISSUES_JSON_BYTES: usize = 65_536;
pub const MAX_REQUEST_BODY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdCollector {
    Ldaps,
    Kerberos,
}

impl fmt::Display for AdCollector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ldaps => "ldaps",
            Self::Kerberos => "kerberos",
        })
    }
}

impl FromStr for AdCollector {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ldaps" => Ok(Self::Ldaps),
            "kerberos" => Ok(Self::Kerberos),
            _ => Err(format!("unknown AD collector: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdCollectionStatus {
    Succeeded,
    Partial,
    Failed,
}

impl fmt::Display for AdCollectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Succeeded => "succeeded",
            Self::Partial => "partial",
            Self::Failed => "failed",
        })
    }
}

impl FromStr for AdCollectionStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "succeeded" => Ok(Self::Succeeded),
            "partial" => Ok(Self::Partial),
            "failed" => Ok(Self::Failed),
            _ => Err(format!("unknown AD collection status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdEntityKind {
    DomainController,
    Site,
    Subnet,
    Trust,
    Computer,
    Group,
    GroupMembership,
}

impl fmt::Display for AdEntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DomainController => "domain_controller",
            Self::Site => "site",
            Self::Subnet => "subnet",
            Self::Trust => "trust",
            Self::Computer => "computer",
            Self::Group => "group",
            Self::GroupMembership => "group_membership",
        })
    }
}

impl FromStr for AdEntityKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "domain_controller" => Ok(Self::DomainController),
            "site" => Ok(Self::Site),
            "subnet" => Ok(Self::Subnet),
            "trust" => Ok(Self::Trust),
            "computer" => Ok(Self::Computer),
            "group" => Ok(Self::Group),
            "group_membership" => Ok(Self::GroupMembership),
            _ => Err(format!("unknown AD entity kind: {value}")),
        }
    }
}

/// A bounded, non-sensitive collection issue. It deliberately has no raw
/// response/attribute field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdCollectionIssue {
    /// Short machine-readable issue category (e.g. `limit_reached`, `collector_failure`).
    pub code: String,
    /// Bounded, pre-approved human-readable summary. Never raw directory/LDAP error text.
    pub message: String,
    /// The entity this issue relates to, if it concerns one specific entity rather than the whole collection.
    pub entity_external_id: Option<String>,
}

/// One normalized AD inventory entity. Unknown fields are rejected to prevent
/// callers from smuggling arbitrary LDAP attributes into persistence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdCollectedEntity {
    /// What kind of directory object this is (domain controller, site, subnet, trust, computer, group, group membership).
    pub kind: AdEntityKind,
    /// Opaque stable identifier (for example objectGUID or a one-way hash),
    /// never a distinguished name or other raw directory attribute.
    pub external_id: String,
    /// Display name for this entity (e.g. computer name, group name).
    pub name: String,
    /// DNS name, when this entity kind has one (e.g. a computer's FQDN).
    pub dns_name: Option<String>,
    /// The `external_id` of this entity's parent, when the relationship is hierarchical (e.g. a group membership's group).
    pub parent_external_id: Option<String>,
    /// The `external_id` of a related entity, when the relationship isn't purely hierarchical (e.g. a trust's remote domain).
    pub related_external_id: Option<String>,
    /// AD site this entity is associated with, when known.
    pub site_name: Option<String>,
    /// Reported operating system name, for computer entities.
    pub operating_system: Option<String>,
    /// Reported operating system version, for computer entities.
    pub operating_system_version: Option<String>,
    /// CIDR notation. Only valid for `subnet` entities.
    pub network_prefix: Option<String>,
    /// Whether the directory object is enabled, when this kind of entity has an enabled/disabled state.
    pub is_enabled: Option<bool>,
    /// When the collector observed this entity.
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdCollectedDomain {
    /// Fully-qualified DNS name of the domain (e.g. `example.test`).
    pub dns_name: String,
    /// DNS name of the forest root domain, if this domain belongs to a multi-domain forest.
    pub forest_dns_name: Option<String>,
    /// Legacy NetBIOS name of the domain (e.g. `EXAMPLE`).
    pub netbios_name: Option<String>,
    /// Domain functional level, as reported by the directory (e.g. `Windows2016Domain`).
    pub functional_level: Option<String>,
    /// When the collector observed this domain.
    pub observed_at: DateTime<Utc>,
    /// Directory entities discovered within this domain.
    pub entities: Vec<AdCollectedEntity>,
}

/// Atomic replacement payload for one server-issued credential target. Only a
/// complete, successful, non-truncated submission may replace inventory.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdCollectionRequest {
    /// Network this collection belongs to.
    pub network_id: Uuid,
    /// The Active Directory credential used to collect this inventory.
    pub credential_id: Uuid,
    /// The host the daemon ran this collection from.
    pub target_host_id: Uuid,
    /// Address the daemon connected to for this collection.
    #[schema(value_type = String)]
    pub target_ip: IpAddr,
    /// The discovery session this collection is part of.
    pub discovery_id: Uuid,
    /// The discovery session's session ID.
    pub session_id: Uuid,
    /// Whether the collection completed fully, partially, or failed.
    pub status: AdCollectionStatus,
    /// When the daemon started this collection.
    pub started_at: DateTime<Utc>,
    /// When the daemon finished this collection.
    pub completed_at: DateTime<Utc>,
    /// Whether one or more directory result limits were reached, so this collection is not a complete inventory.
    #[serde(default)]
    pub truncated: bool,
    /// Bounded, non-sensitive issues encountered during collection.
    #[serde(default)]
    pub issues: Vec<AdCollectionIssue>,
    /// Domains and their entities discovered by this collection.
    #[serde(default)]
    pub domains: Vec<AdCollectedDomain>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdCollectionRun {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Organization this collection run belongs to.
    pub organization_id: Uuid,
    /// Network this collection run belongs to.
    pub network_id: Uuid,
    /// The daemon that performed this collection, if known.
    pub daemon_id: Option<Uuid>,
    /// The Active Directory credential used, if known.
    pub credential_id: Option<Uuid>,
    /// The host the daemon ran this collection from, if known.
    pub target_host_id: Option<Uuid>,
    /// Address the daemon connected to for this collection.
    #[schema(value_type = String)]
    pub target_ip: IpAddr,
    /// The discovery session this collection is part of, if known.
    pub discovery_id: Option<Uuid>,
    /// The discovery session's session ID.
    pub session_id: Uuid,
    /// Idempotency key identifying this specific collection submission.
    pub collection_key: String,
    /// Which transport collected this run (LDAPS password bind or Kerberos).
    pub collector: AdCollector,
    /// Whether the collection completed fully, partially, or failed.
    pub status: AdCollectionStatus,
    /// When the daemon started this collection.
    pub started_at: DateTime<Utc>,
    /// When the daemon finished this collection.
    pub completed_at: DateTime<Utc>,
    /// Number of domains this run discovered.
    pub domain_count: u32,
    /// Number of directory entities this run discovered, across all domains.
    pub entity_count: u32,
    /// Whether one or more directory result limits were reached, so this collection is not a complete inventory.
    pub truncated: bool,
    /// Whether this run's inventory replaced the network's stored Active Directory inventory.
    pub inventory_applied: bool,
    /// Bounded, non-sensitive issues encountered during collection.
    pub issues: Vec<AdCollectionIssue>,
    /// When this run was recorded on the server.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, sqlx::FromRow)]
pub struct AdDomain {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Organization this domain belongs to.
    pub organization_id: Uuid,
    /// Network this domain belongs to.
    pub network_id: Uuid,
    /// Idempotency key of the collection run that most recently wrote this domain.
    pub collection_key: String,
    /// Fully-qualified DNS name of the domain (e.g. `example.test`).
    pub dns_name: String,
    /// DNS name of the forest root domain, if this domain belongs to a multi-domain forest.
    pub forest_dns_name: Option<String>,
    /// Legacy NetBIOS name of the domain (e.g. `EXAMPLE`).
    pub netbios_name: Option<String>,
    /// Domain functional level, as reported by the directory (e.g. `Windows2016Domain`).
    pub functional_level: Option<String>,
    /// The collection run that most recently wrote this domain's stored inventory.
    pub last_collection_run_id: Uuid,
    /// When the collector observed this domain.
    pub observed_at: DateTime<Utc>,
    /// When this domain was first stored.
    pub created_at: DateTime<Utc>,
    /// When this domain was last updated.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AdEntity {
    /// Server-assigned unique identifier.
    pub id: Uuid,
    /// Organization this entity belongs to.
    pub organization_id: Uuid,
    /// Network this entity belongs to.
    pub network_id: Uuid,
    /// The domain this entity was collected from.
    pub domain_id: Uuid,
    /// The collection run that most recently wrote this entity.
    pub collection_run_id: Uuid,
    /// What kind of directory object this is (domain controller, site, subnet, trust, computer, group, group membership).
    pub kind: AdEntityKind,
    /// Opaque stable identifier (for example objectGUID or a one-way hash),
    /// never a distinguished name or other raw directory attribute.
    pub external_id: String,
    /// Display name for this entity (e.g. computer name, group name).
    pub name: String,
    /// DNS name, when this entity kind has one (e.g. a computer's FQDN).
    pub dns_name: Option<String>,
    /// The `external_id` of this entity's parent, when the relationship is hierarchical (e.g. a group membership's group).
    pub parent_external_id: Option<String>,
    /// The `external_id` of a related entity, when the relationship isn't purely hierarchical (e.g. a trust's remote domain).
    pub related_external_id: Option<String>,
    /// AD site this entity is associated with, when known.
    pub site_name: Option<String>,
    /// Reported operating system name, for computer entities.
    pub operating_system: Option<String>,
    /// Reported operating system version, for computer entities.
    pub operating_system_version: Option<String>,
    /// CIDR notation. Only valid for `subnet` entities.
    pub network_prefix: Option<String>,
    /// Whether the directory object is enabled, when this kind of entity has an enabled/disabled state.
    pub is_enabled: Option<bool>,
    /// When the collector observed this entity.
    pub observed_at: DateTime<Utc>,
    /// When this entity was first stored.
    pub created_at: DateTime<Utc>,
    /// When this entity was last updated.
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct AdListQuery {
    /// Restrict results to this network. Omit to list across every network the caller can access.
    pub network_id: Option<Uuid>,
    /// Maximum number of rows to return (1-500, default 100).
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
    /// Number of rows to skip, for pagination.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct AdEntityListQuery {
    /// Restrict results to this network. Omit to list across every network the caller can access.
    pub network_id: Option<Uuid>,
    /// Restrict results to entities collected from this Active Directory domain.
    pub domain_id: Option<Uuid>,
    /// Restrict results to entities of this kind (e.g. computer, user).
    pub kind: Option<AdEntityKind>,
    /// Maximum number of rows to return (1-500, default 100).
    #[param(minimum = 1, maximum = 500)]
    pub limit: Option<u32>,
    /// Number of rows to skip, for pagination.
    #[param(minimum = 0)]
    pub offset: Option<u32>,
}

impl AdListQuery {
    pub fn pagination(&self) -> (u32, u32) {
        (
            self.limit.unwrap_or(100).clamp(1, 500),
            self.offset.unwrap_or(0),
        )
    }
}

impl AdEntityListQuery {
    pub fn pagination(&self) -> (u32, u32) {
        (
            self.limit.unwrap_or(100).clamp(1, 500),
            self.offset.unwrap_or(0),
        )
    }
}

impl AdCollectionRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.network_id.is_nil()
            || self.credential_id.is_nil()
            || self.target_host_id.is_nil()
            || self.discovery_id.is_nil()
            || self.session_id.is_nil()
        {
            return Err("AD collection identity UUIDs must not be nil".into());
        }
        let clock_skew_limit = Utc::now() + Duration::minutes(5);
        if self.started_at > clock_skew_limit || self.completed_at > clock_skew_limit {
            return Err(
                "collection timestamps must not be more than 5 minutes in the future".into(),
            );
        }
        if self.completed_at < self.started_at {
            return Err("completed_at must not be before started_at".into());
        }
        if self.completed_at - self.started_at > Duration::hours(24) {
            return Err("collection duration must not exceed 24 hours".into());
        }
        if self.domains.len() > MAX_DOMAINS_PER_COLLECTION {
            return Err(format!(
                "at most {MAX_DOMAINS_PER_COLLECTION} domains are allowed"
            ));
        }
        if self.issues.len() > MAX_ISSUES_PER_COLLECTION {
            return Err(format!(
                "at most {MAX_ISSUES_PER_COLLECTION} issues are allowed"
            ));
        }
        let issues_json_bytes = serde_json::to_vec(&self.issues)
            .map_err(|_| "issues could not be serialized".to_string())?
            .len();
        if issues_json_bytes > MAX_ISSUES_JSON_BYTES {
            return Err(format!(
                "serialized issues must not exceed {MAX_ISSUES_JSON_BYTES} bytes"
            ));
        }
        let entity_count = self
            .domains
            .iter()
            .map(|domain| domain.entities.len())
            .sum::<usize>();
        if entity_count > MAX_ENTITIES_PER_COLLECTION {
            return Err(format!(
                "at most {MAX_ENTITIES_PER_COLLECTION} entities are allowed"
            ));
        }
        let request_bytes = serde_json::to_vec(self)
            .map_err(|_| "collection request could not be serialized".to_string())?
            .len();
        if request_bytes > MAX_REQUEST_BODY_BYTES {
            return Err(format!(
                "serialized collection request must not exceed {MAX_REQUEST_BODY_BYTES} bytes"
            ));
        }

        for issue in &self.issues {
            bounded("issue.code", &issue.code, 1, 64)?;
            bounded("issue.message", &issue.message, 1, 512)?;
            let approved = matches!(
                (issue.code.as_str(), issue.message.as_str()),
                (
                    "limit_reached",
                    "One or more directory result limits were reached; inventory is partial."
                ) | (
                    "collector_failure",
                    "Directory collection failed before a complete inventory was produced."
                )
            );
            if !approved || issue.entity_external_id.is_some() {
                return Err("collection issues must use an approved bounded summary".into());
            }
            if let Some(external_id) = issue.entity_external_id.as_deref() {
                validate_opaque_id("issue.entity_external_id", external_id)?;
            }
        }
        match self.status {
            AdCollectionStatus::Succeeded
                if self.truncated || !self.issues.is_empty() || self.domains.is_empty() =>
            {
                return Err(
                    "successful collections must be complete, issue-free, and contain a domain"
                        .into(),
                );
            }
            AdCollectionStatus::Partial if !self.truncated => {
                return Err("partial collections must be marked truncated".into());
            }
            AdCollectionStatus::Failed
                if !self.truncated || !self.domains.is_empty() || self.issues.is_empty() =>
            {
                return Err(
                    "failed collections must be truncated, contain no inventory, and include an issue"
                        .into(),
                );
            }
            _ => {}
        }
        let mut domain_keys = HashSet::with_capacity(self.domains.len());
        for domain in &self.domains {
            validate_dns_name("domain.dns_name", &domain.dns_name)?;
            validate_observed_at("domain.observed_at", domain.observed_at, self)?;
            if !domain_keys.insert(domain.dns_name.to_ascii_lowercase()) {
                return Err(format!("duplicate domain dns_name: {}", domain.dns_name));
            }
            optional_dns_name("domain.forest_dns_name", domain.forest_dns_name.as_deref())?;
            optional_bounded("domain.netbios_name", domain.netbios_name.as_deref(), 64)?;
            optional_bounded(
                "domain.functional_level",
                domain.functional_level.as_deref(),
                100,
            )?;
            let mut entity_keys = HashSet::with_capacity(domain.entities.len());
            for entity in &domain.entities {
                validate_opaque_id("entity.external_id", &entity.external_id)?;
                validate_observed_at("entity.observed_at", entity.observed_at, self)?;
                if !entity_keys.insert((entity.kind, entity.external_id.clone())) {
                    return Err(format!(
                        "duplicate {:?} entity external_id in {}: {}",
                        entity.kind, domain.dns_name, entity.external_id
                    ));
                }
                bounded("entity.name", &entity.name, 1, 256)?;
                optional_dns_name("entity.dns_name", entity.dns_name.as_deref())?;
                if let Some(external_id) = entity.parent_external_id.as_deref() {
                    validate_opaque_id("entity.parent_external_id", external_id)?;
                }
                if let Some(external_id) = entity.related_external_id.as_deref() {
                    validate_opaque_id("entity.related_external_id", external_id)?;
                }
                optional_bounded("entity.site_name", entity.site_name.as_deref(), 256)?;
                optional_bounded(
                    "entity.operating_system",
                    entity.operating_system.as_deref(),
                    256,
                )?;
                optional_bounded(
                    "entity.operating_system_version",
                    entity.operating_system_version.as_deref(),
                    128,
                )?;
                match (entity.kind, entity.network_prefix.as_deref()) {
                    (AdEntityKind::Subnet, Some(prefix)) => {
                        prefix.parse::<IpNetwork>().map_err(|_| {
                            format!("entity.network_prefix is not valid CIDR: {prefix}")
                        })?;
                    }
                    (AdEntityKind::Subnet, None) => {
                        return Err("subnet entities require network_prefix".into());
                    }
                    (_, Some(_)) => {
                        return Err("network_prefix is only valid for subnet entities".into());
                    }
                    _ => {}
                }
                if matches!(
                    entity.kind,
                    AdEntityKind::Trust | AdEntityKind::GroupMembership
                ) && entity.related_external_id.is_none()
                {
                    return Err(
                        "trust and group_membership entities require related_external_id".into(),
                    );
                }
                if entity.kind == AdEntityKind::GroupMembership
                    && entity.parent_external_id.is_none()
                {
                    return Err("group_membership entities require parent_external_id".into());
                }
            }
        }
        Ok(())
    }
}

fn validate_observed_at(
    field: &str,
    value: DateTime<Utc>,
    request: &AdCollectionRequest,
) -> Result<(), String> {
    let tolerance = Duration::minutes(5);
    if value < request.started_at - tolerance || value > request.completed_at + tolerance {
        return Err(format!(
            "{field} must fall within 5 minutes of the collection window"
        ));
    }
    Ok(())
}

fn bounded(field: &str, value: &str, min: usize, max: usize) -> Result<(), String> {
    let length = value.chars().count();
    if value.trim() != value || value.chars().any(char::is_control) || length < min || length > max
    {
        return Err(format!(
            "{field} must be trimmed, contain no control characters, and be between {min} and {max} characters"
        ));
    }
    Ok(())
}

fn optional_bounded(field: &str, value: Option<&str>, max: usize) -> Result<(), String> {
    if let Some(value) = value {
        bounded(field, value, 1, max)?;
    }
    Ok(())
}

fn validate_opaque_id(field: &str, value: &str) -> Result<(), String> {
    bounded(field, value, 1, 512)?;
    // LDAP distinguished names are raw directory attributes and can disclose
    // directory structure. Relationships must use objectGUIDs or opaque hashes.
    if value
        .split(',')
        .any(|component| component.trim().split_once('=').is_some())
    {
        return Err(format!(
            "{field} must be an opaque identifier, not a distinguished name"
        ));
    }
    Ok(())
}

fn optional_dns_name(field: &str, value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        validate_dns_name(field, value)?;
    }
    Ok(())
}

fn validate_dns_name(field: &str, value: &str) -> Result<(), String> {
    bounded(field, value, 1, 253)?;
    if value.ends_with('.')
        || value.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return Err(format!("{field} must be a valid DNS name"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> AdCollectionRequest {
        AdCollectionRequest {
            network_id: Uuid::new_v4(),
            credential_id: Uuid::new_v4(),
            target_host_id: Uuid::new_v4(),
            target_ip: "192.0.2.10".parse().unwrap(),
            discovery_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            status: AdCollectionStatus::Succeeded,
            started_at: Utc::now() - Duration::minutes(1),
            completed_at: Utc::now(),
            truncated: false,
            issues: vec![],
            domains: vec![AdCollectedDomain {
                dns_name: "example.test".into(),
                forest_dns_name: Some("example.test".into()),
                netbios_name: Some("EXAMPLE".into()),
                functional_level: Some("Windows2016Domain".into()),
                observed_at: Utc::now(),
                entities: vec![AdCollectedEntity {
                    kind: AdEntityKind::Computer,
                    external_id: Uuid::new_v4().to_string(),
                    name: "workstation-01".into(),
                    dns_name: Some("workstation-01.example.test".into()),
                    parent_external_id: None,
                    related_external_id: None,
                    site_name: Some("HeadOffice".into()),
                    operating_system: Some("Windows 11".into()),
                    operating_system_version: Some("10.0".into()),
                    network_prefix: None,
                    is_enabled: Some(true),
                    observed_at: Utc::now(),
                }],
            }],
        }
    }

    #[test]
    fn collection_rejects_control_characters_in_persisted_text() {
        let mut value = request();
        value.domains[0].entities[0].name = "server\nforged-log".to_string();
        assert!(value.validate().unwrap_err().contains("control characters"));
    }

    #[test]
    fn accepts_normalized_inventory() {
        assert_eq!(request().validate(), Ok(()));
    }

    #[test]
    fn rejects_unbounded_or_misplaced_topology_fields() {
        let mut value = request();
        value.domains[0].entities[0].network_prefix = Some("10.0.0.0/24".into());
        assert_eq!(
            value.validate(),
            Err("network_prefix is only valid for subnet entities".into())
        );

        let mut value = request();
        value.issues.push(AdCollectionIssue {
            code: "ldap".into(),
            message: "x".repeat(513),
            entity_external_id: None,
        });
        assert!(value.validate().unwrap_err().contains("issue.message"));
    }

    #[test]
    fn enforces_entity_and_serialized_body_caps() {
        let mut too_many = request();
        let template = too_many.domains[0].entities[0].clone();
        too_many.domains[0].entities = (0..=MAX_ENTITIES_PER_COLLECTION)
            .map(|index| AdCollectedEntity {
                external_id: format!("entity-{index}"),
                ..template.clone()
            })
            .collect();
        assert!(too_many.validate().unwrap_err().contains("at most 3000"));

        let mut oversized = request();
        oversized.domains[0].entities = (0..MAX_ENTITIES_PER_COLLECTION)
            .map(|index| AdCollectedEntity {
                external_id: format!("entity-{index}"),
                name: "🦀".repeat(256),
                site_name: Some("🦀".repeat(256)),
                operating_system: Some("🦀".repeat(256)),
                operating_system_version: Some("🦀".repeat(128)),
                ..template.clone()
            })
            .collect();
        assert!(
            oversized
                .validate()
                .unwrap_err()
                .contains("serialized collection request")
        );
    }

    #[test]
    fn failed_collection_payload_is_valid_but_not_replaceable() {
        let mut value = request();
        value.status = AdCollectionStatus::Failed;
        value.truncated = true;
        value.domains.clear();
        value.issues.push(AdCollectionIssue {
            code: "collector_failure".into(),
            message: "Directory collection failed before a complete inventory was produced.".into(),
            entity_external_id: None,
        });
        assert_eq!(value.validate(), Ok(()));
        assert!(!value.replaces_inventory());

        let mut value = request();
        value.status = AdCollectionStatus::Partial;
        value.truncated = true;
        value.issues.push(AdCollectionIssue {
            code: "limit_reached".into(),
            message: "One or more directory result limits were reached; inventory is partial."
                .into(),
            entity_external_id: None,
        });
        assert_eq!(value.validate(), Ok(()));
        assert!(!value.replaces_inventory());

        let mut value = request();
        value.status = AdCollectionStatus::Succeeded;
        value.truncated = true;
        assert!(value.validate().is_err());
        assert!(!value.replaces_inventory());
    }

    #[test]
    fn rejects_unapproved_issue_text_and_inconsistent_status() {
        let mut value = request();
        value.issues.push(AdCollectionIssue {
            code: "ldap_error".into(),
            message: "CN=person,DC=example,DC=test".into(),
            entity_external_id: None,
        });
        assert!(
            value
                .validate()
                .unwrap_err()
                .contains("approved bounded summary")
        );

        let mut value = request();
        value.domains.clear();
        assert!(value.validate().unwrap_err().contains("contain a domain"));
    }

    #[test]
    fn rejects_unknown_raw_ldap_fields() {
        let json = serde_json::json!({
            "kind": "computer",
            "external_id": "id",
            "name": "pc",
            "dns_name": null,
            "parent_external_id": null,
            "related_external_id": null,
            "site_name": null,
            "operating_system": null,
            "operating_system_version": null,
            "network_prefix": null,
            "is_enabled": true,
            "observed_at": Utc::now(),
            "raw_ldap_attributes": { "ms-Mcs-AdmPwd": "must-not-persist" }
        });
        let error = serde_json::from_value::<AdCollectedEntity>(json).unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn rejects_distinguished_names_as_persisted_identifiers() {
        let mut value = request();
        value.domains[0].entities[0].external_id =
            "CN=workstation-01,OU=Computers,DC=example,DC=test".into();
        assert!(
            value
                .validate()
                .unwrap_err()
                .contains("not a distinguished name")
        );
    }

    #[test]
    fn rejects_oversized_multibyte_issue_json() {
        let mut value = request();
        value.issues = (0..MAX_ISSUES_PER_COLLECTION)
            .map(|_| AdCollectionIssue {
                code: "ldap".into(),
                message: "🦀".repeat(512),
                entity_external_id: None,
            })
            .collect();
        assert!(value.validate().unwrap_err().contains("serialized issues"));
    }
}

impl AdCollectionRequest {
    pub fn replaces_inventory(&self) -> bool {
        self.status == AdCollectionStatus::Succeeded && !self.truncated
    }
}
