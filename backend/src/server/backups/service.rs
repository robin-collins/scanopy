use std::collections::BTreeSet;
use std::io::{Cursor, Write};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/// Logical portions of an organization backup. `complete` selects every portion.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupSection {
    Complete,
    Topology,
    Scans,
    Daemons,
    Networks,
    Subnets,
    Hosts,
    Services,
    Categories,
    Tags,
    Users,
    ApiKeys,
    Credentials,
    Settings,
}

impl BackupSection {
    pub const ALL: [Self; 13] = [
        Self::Topology,
        Self::Scans,
        Self::Daemons,
        Self::Networks,
        Self::Subnets,
        Self::Hosts,
        Self::Services,
        Self::Categories,
        Self::Tags,
        Self::Users,
        Self::ApiKeys,
        Self::Credentials,
        Self::Settings,
    ];
}

#[derive(Clone, Copy)]
struct TableExport {
    name: &'static str,
    section: BackupSection,
    predicate: &'static str,
}

// This explicit inventory is intentional: adding a table requires deciding how it is tenant
// scoped instead of risking a cross-organization export through generic schema introspection.
const TABLES: &[TableExport] = &[
    TableExport {
        name: "organizations",
        section: BackupSection::Settings,
        predicate: "t.id = $1",
    },
    TableExport {
        name: "users",
        section: BackupSection::Users,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "user_network_access",
        section: BackupSection::Users,
        predicate: "EXISTS (SELECT 1 FROM users u WHERE u.id = t.user_id AND u.organization_id = $1)",
    },
    TableExport {
        name: "user_api_keys",
        section: BackupSection::ApiKeys,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "user_api_key_network_access",
        section: BackupSection::ApiKeys,
        predicate: "EXISTS (SELECT 1 FROM user_api_keys k WHERE k.id = t.api_key_id AND k.organization_id = $1)",
    },
    TableExport {
        name: "api_keys",
        section: BackupSection::ApiKeys,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "networks",
        section: BackupSection::Networks,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "daemons",
        section: BackupSection::Daemons,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "daemon_interfaced_subnets",
        section: BackupSection::Daemons,
        predicate: "EXISTS (SELECT 1 FROM daemons d JOIN networks n ON n.id = d.network_id WHERE d.id = t.daemon_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "discovery",
        section: BackupSection::Scans,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "snapshots",
        section: BackupSection::Scans,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "topologies",
        section: BackupSection::Topology,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "topology_node_positions",
        section: BackupSection::Topology,
        predicate: "EXISTS (SELECT 1 FROM topologies x JOIN networks n ON n.id = x.network_id WHERE x.id = t.topology_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "custom_topology_views",
        section: BackupSection::Topology,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "custom_topology_view_nodes",
        section: BackupSection::Topology,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "custom_topology_view_edges",
        section: BackupSection::Topology,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "subnets",
        section: BackupSection::Subnets,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "vlans",
        section: BackupSection::Subnets,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "subnet_vlans",
        section: BackupSection::Subnets,
        predicate: "EXISTS (SELECT 1 FROM subnets s JOIN networks n ON n.id = s.network_id WHERE s.id = t.subnet_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "hosts",
        section: BackupSection::Hosts,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "host_images",
        section: BackupSection::Hosts,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "ip_addresses",
        section: BackupSection::Hosts,
        predicate: "EXISTS (SELECT 1 FROM hosts h JOIN networks n ON n.id = h.network_id WHERE h.id = t.host_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "interfaces",
        section: BackupSection::Hosts,
        predicate: "EXISTS (SELECT 1 FROM hosts h JOIN networks n ON n.id = h.network_id WHERE h.id = t.host_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "ports",
        section: BackupSection::Services,
        predicate: "EXISTS (SELECT 1 FROM hosts h JOIN networks n ON n.id = h.network_id WHERE h.id = t.host_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "services",
        section: BackupSection::Services,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "custom_known_ports",
        section: BackupSection::Services,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "bindings",
        section: BackupSection::Services,
        predicate: "EXISTS (SELECT 1 FROM services s JOIN networks n ON n.id = s.network_id WHERE s.id = t.service_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "dependencies",
        section: BackupSection::Services,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "dependency_members",
        section: BackupSection::Services,
        predicate: "EXISTS (SELECT 1 FROM dependencies d JOIN networks n ON n.id = d.network_id WHERE d.id = t.dependency_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "categories",
        section: BackupSection::Categories,
        predicate: "t.organization_id = $1 OR t.organization_id IS NULL",
    },
    TableExport {
        name: "tags",
        section: BackupSection::Tags,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "entity_tags",
        section: BackupSection::Tags,
        predicate: "EXISTS (SELECT 1 FROM tags x WHERE x.id = t.tag_id AND x.organization_id = $1)",
    },
    TableExport {
        name: "credentials",
        section: BackupSection::Credentials,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "network_credentials",
        section: BackupSection::Credentials,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "host_credentials",
        section: BackupSection::Credentials,
        predicate: "EXISTS (SELECT 1 FROM hosts h JOIN networks n ON n.id = h.network_id WHERE h.id = t.host_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "ad_domains",
        section: BackupSection::Networks,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "ad_collection_runs",
        section: BackupSection::Scans,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "ad_entities",
        section: BackupSection::Hosts,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "passive_observations",
        section: BackupSection::Scans,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "passive_correlations",
        section: BackupSection::Scans,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
    TableExport {
        name: "library_objects",
        section: BackupSection::Settings,
        predicate: "t.organization_id = $1 OR t.organization_id IS NULL",
    },
    TableExport {
        name: "custom_service_definitions",
        section: BackupSection::Settings,
        predicate: "TRUE",
    },
    TableExport {
        name: "invites",
        section: BackupSection::Users,
        predicate: "t.organization_id = $1",
    },
    TableExport {
        name: "shares",
        section: BackupSection::Settings,
        predicate: "EXISTS (SELECT 1 FROM networks n WHERE n.id = t.network_id AND n.organization_id = $1)",
    },
];

// Framework-owned tables are deliberately not portable application state. SQLx tracks applied
// migrations here, while tower-sessions contains short-lived login sessions that must not survive
// a restore onto another server.
#[cfg(test)]
const EXCLUDED_SYSTEM_TABLES: &[&str] = &["_sqlx_migrations", "tower_sessions"];

#[derive(Debug, Serialize)]
struct BackupManifest {
    format_version: u32,
    server_version: &'static str,
    created_at: DateTime<Utc>,
    organization_id: Uuid,
    sections: BTreeSet<BackupSection>,
    tables: Vec<ManifestTable>,
}

#[derive(Debug, Serialize)]
struct ManifestTable {
    name: &'static str,
    row_count: usize,
    file: String,
}

fn selected_tables(sections: &BTreeSet<BackupSection>) -> Vec<TableExport> {
    let complete = sections.contains(&BackupSection::Complete);
    TABLES
        .iter()
        .copied()
        .filter(|table| complete || sections.contains(&table.section))
        .collect()
}

pub async fn create_backup(
    pool: &PgPool,
    organization_id: Uuid,
    sections: BTreeSet<BackupSection>,
) -> anyhow::Result<Vec<u8>> {
    let tables = selected_tables(&sections);
    let mut exported = Vec::with_capacity(tables.len());

    // A repeatable-read transaction guarantees that every JSON file describes the same point in
    // time, even while scans continue writing to the database.
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    for table in tables {
        let sql = format!(
            "SELECT COALESCE(jsonb_agg(exported.row), '[]'::jsonb) AS rows \
             FROM (SELECT to_jsonb(t) AS row FROM {} t WHERE {}) exported",
            table.name, table.predicate
        );
        let value: Value = sqlx::query(&sql)
            .bind(organization_id)
            .fetch_one(&mut *tx)
            .await?
            .try_get("rows")?;
        exported.push((table.name, value));
    }
    tx.commit().await?;

    build_archive(organization_id, sections, exported)
}

fn build_archive(
    organization_id: Uuid,
    sections: BTreeSet<BackupSection>,
    exported: Vec<(&'static str, Value)>,
) -> anyhow::Result<Vec<u8>> {
    let mut manifest = BackupManifest {
        format_version: 1,
        server_version: env!("CARGO_PKG_VERSION"),
        created_at: Utc::now(),
        organization_id,
        sections,
        tables: Vec::with_capacity(exported.len()),
    };
    let mut output = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    for (name, rows) in exported {
        let file = format!("tables/{name}.json");
        let row_count = rows.as_array().map_or(0, Vec::len);
        zip.start_file(&file, options)?;
        zip.write_all(&serde_json::to_vec_pretty(&rows)?)?;
        manifest.tables.push(ManifestTable {
            name,
            row_count,
            file,
        });
    }
    zip.start_file("manifest.json", options)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    zip.finish()?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn complete_selects_every_table_once() {
        let selected = selected_tables(&BTreeSet::from([BackupSection::Complete]));
        let unique = selected.iter().map(|t| t.name).collect::<BTreeSet<_>>();
        assert_eq!(selected.len(), TABLES.len());
        assert_eq!(unique.len(), TABLES.len());
    }

    #[test]
    fn services_backup_includes_org_scoped_custom_known_ports() {
        let selected = selected_tables(&BTreeSet::from([BackupSection::Services]));
        let known_ports = selected
            .iter()
            .find(|table| table.name == "custom_known_ports")
            .expect("Services backup must retain custom Known Ports");

        assert_eq!(known_ports.predicate, "t.organization_id = $1");
    }

    #[test]
    fn archive_contains_individual_json_files_and_manifest() {
        let bytes = build_archive(
            Uuid::nil(),
            BTreeSet::from([BackupSection::Tags]),
            vec![("tags", serde_json::json!([{"id": Uuid::nil()}]))],
        )
        .unwrap();
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        assert!(zip.by_name("tables/tags.json").is_ok());
        let mut manifest = String::new();
        zip.by_name("manifest.json")
            .unwrap()
            .read_to_string(&mut manifest)
            .unwrap();
        let manifest: Value = serde_json::from_str(&manifest).unwrap();
        assert_eq!(manifest["format_version"], 1);
        assert_eq!(manifest["tables"][0]["row_count"], 1);
    }

    #[tokio::test]
    async fn complete_backup_executes_every_application_table_export() {
        let (pool, _database_url, _container) = crate::tests::setup_test_db().await;
        crate::server::shared::storage::migration_runner::apply_migrations(
            &pool,
            std::path::Path::new("./migrations"),
        )
        .await
        .expect("failed to apply migrations");

        // Running the export against the real migrated schema verifies every predicate and table
        // name, even though this randomly selected organization has no rows.
        let archive = create_backup(
            &pool,
            Uuid::new_v4(),
            BTreeSet::from([BackupSection::Complete]),
        )
        .await
        .expect("complete backup should match the migrated schema");
        assert!(!archive.is_empty());

        let database_tables: BTreeSet<String> = sqlx::query_scalar(
            "SELECT table_name FROM information_schema.tables
             WHERE table_schema = 'public' AND table_type = 'BASE TABLE'",
        )
        .fetch_all(&pool)
        .await
        .expect("failed to inspect migrated schema")
        .into_iter()
        .collect();
        let exported_tables: BTreeSet<String> =
            TABLES.iter().map(|table| table.name.to_owned()).collect();
        let missing: Vec<_> = database_tables
            .difference(&exported_tables)
            .filter(|table| !EXCLUDED_SYSTEM_TABLES.contains(&table.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "application tables missing from complete backup: {missing:?}"
        );
    }
}
