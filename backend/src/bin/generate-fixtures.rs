//! Generates metadata fixture JSON files for the frontend.
//!
//! Run with: cargo run --bin generate-fixtures
//! Or via: make generate-fixtures

use scanopy::server::billing::plans::get_website_fixture_plans;
use scanopy::server::billing::types::base::BillingPlan;
use scanopy::server::billing::types::features::Feature;
use scanopy::server::credentials::r#impl::types::CredentialTypeVariant;
use scanopy::server::discovery::r#impl::types::DiscoveryType;
use scanopy::server::groups::r#impl::types::GroupType;
use scanopy::server::ports::r#impl::base::PortType;
use scanopy::server::services::definitions::ServiceDefinitionRegistry;
use scanopy::server::shared::concepts::Concept;
use scanopy::server::shared::entities::EntityDiscriminants;
use scanopy::server::shared::types::metadata::{EntityMetadata, MetadataProvider, TypeMetadata};
use scanopy::server::subnets::r#impl::types::SubnetType;
use scanopy::server::topology::types::edges::EdgeType;
use scanopy::server::users::r#impl::permissions::UserOrgPermissions;
use std::fs;
use std::path::PathBuf;
use strum::{IntoDiscriminant, IntoEnumIterator};

fn main() {
    let output_dir = parse_output_dir();

    println!(
        "Generating metadata fixtures to {}...",
        output_dir.display()
    );

    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Billing fixtures — website plans (curated, for billing modal)
    let plan_metadata: Vec<TypeMetadata> = get_website_fixture_plans()
        .iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&plan_metadata, &output_dir, "billing-plans.json");

    // All billing plan variants (for metadata store, matches old /api/metadata)
    let all_plan_metadata: Vec<TypeMetadata> =
        BillingPlan::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&all_plan_metadata, &output_dir, "billing-plans-all.json");

    let feature_metadata: Vec<TypeMetadata> = Feature::iter().map(|f| f.to_metadata()).collect();
    write_fixture(&feature_metadata, &output_dir, "features.json");

    let service_defs: Vec<TypeMetadata> = ServiceDefinitionRegistry::all_service_definitions()
        .iter()
        .map(|t| t.to_metadata())
        .collect();
    write_fixture(&service_defs, &output_dir, "service-definitions.json");

    let subnet_types: Vec<TypeMetadata> = SubnetType::iter().map(|t| t.to_metadata()).collect();
    write_fixture(&subnet_types, &output_dir, "subnet-types.json");

    let edge_types: Vec<TypeMetadata> = EdgeType::iter().map(|t| t.to_metadata()).collect();
    write_fixture(&edge_types, &output_dir, "edge-types.json");

    let group_types: Vec<TypeMetadata> = GroupType::iter()
        .map(|t| t.discriminant().to_metadata())
        .collect();
    write_fixture(&group_types, &output_dir, "group-types.json");

    let ports: Vec<TypeMetadata> = PortType::iter().map(|p| p.to_metadata()).collect();
    write_fixture(&ports, &output_dir, "ports.json");

    let discovery_types: Vec<TypeMetadata> =
        DiscoveryType::iter().map(|d| d.to_metadata()).collect();
    write_fixture(&discovery_types, &output_dir, "discovery-types.json");

    let permissions: Vec<TypeMetadata> = UserOrgPermissions::iter()
        .map(|p| p.to_metadata())
        .collect();
    write_fixture(&permissions, &output_dir, "permissions.json");

    // EntityMetadata categories
    let entities: Vec<EntityMetadata> = EntityDiscriminants::iter()
        .map(|e| e.to_metadata())
        .collect();
    write_fixture(&entities, &output_dir, "entities.json");

    let concepts: Vec<EntityMetadata> = Concept::iter().map(|e| e.to_metadata()).collect();
    write_fixture(&concepts, &output_dir, "concepts.json");

    let credential_types: Vec<TypeMetadata> = CredentialTypeVariant::iter()
        .map(|v| v.to_credential_type().to_metadata())
        .collect();
    write_fixture(&credential_types, &output_dir, "credential-types.json");

    println!("Done! Generated all metadata fixtures.");
}

fn parse_output_dir() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    for i in 1..args.len() {
        if args[i] == "--output-dir" {
            if let Some(dir) = args.get(i + 1) {
                return PathBuf::from(dir);
            }
            eprintln!("Error: --output-dir requires a path argument");
            std::process::exit(1);
        }
    }
    PathBuf::from("../ui/src/lib/data")
}

fn write_fixture<T: serde::Serialize>(items: &[T], output_dir: &PathBuf, filename: &str) {
    let json = serde_json::to_string_pretty(items).expect("Failed to serialize");
    let path = output_dir.join(filename);
    fs::write(&path, json).unwrap_or_else(|_| panic!("Failed to write {filename}"));
    println!("  {}", filename);
}
