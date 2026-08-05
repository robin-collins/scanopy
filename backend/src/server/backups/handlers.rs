use std::collections::BTreeSet;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use chrono::Utc;
use serde::Deserialize;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::server::auth::middleware::permissions::{Authorized, Owner};
use crate::server::backups::service::{BackupSection, create_backup};
use crate::server::config::AppState;
use crate::server::shared::types::api::{ApiError, ApiResult};

#[derive(Debug, Deserialize, IntoParams)]
struct ExportQuery {
    /// Comma-separated sections. Omit to create a complete backup.
    #[param(example = "hosts,services,tags")]
    sections: Option<String>,
}

pub fn create_router() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new().routes(routes!(export_backup))
}

#[utoipa::path(
    get,
    path = "/export",
    tag = "backups",
    operation_id = "export_backup",
    params(ExportQuery),
    responses((status = 200, description = "ZIP archive with one JSON file per table", content_type = "application/zip", body = [u8])),
    security(("user_api_key" = []), ("session" = []))
)]
async fn export_backup(
    State(state): State<Arc<AppState>>,
    auth: Authorized<Owner>,
    Query(query): Query<ExportQuery>,
) -> ApiResult<impl IntoResponse> {
    let organization_id = auth.require_organization_id()?;
    let sections = parse_sections(query.sections.as_deref())?;
    let archive = create_backup(&state.pool, organization_id, sections)
        .await
        .map_err(|error| ApiError::internal_error(&format!("Failed to create backup: {error}")))?;

    let filename = format!("scanopy-backup-{}.zip", Utc::now().format("%Y%m%d-%H%M%S"));
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/zip"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .map_err(|error| ApiError::internal_error(&error.to_string()))?,
    );
    Ok((headers, Body::from(archive)))
}

fn parse_sections(value: Option<&str>) -> ApiResult<BTreeSet<BackupSection>> {
    let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
        return Ok(BTreeSet::from([BackupSection::Complete]));
    };
    value
        .split(',')
        .map(|raw| {
            serde_json::from_value(serde_json::Value::String(raw.trim().to_ascii_lowercase()))
                .map_err(|_| ApiError::bad_request(&format!("Unknown backup section: {raw}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omitted_sections_means_complete() {
        assert_eq!(
            parse_sections(None).unwrap(),
            BTreeSet::from([BackupSection::Complete])
        );
    }

    #[test]
    fn parses_fine_grained_sections() {
        assert_eq!(
            parse_sections(Some("hosts, services,settings")).unwrap(),
            BTreeSet::from([
                BackupSection::Hosts,
                BackupSection::Services,
                BackupSection::Settings
            ])
        );
    }

    #[test]
    fn rejects_unknown_sections() {
        assert!(parse_sections(Some("hosts,secrets")).is_err());
    }
}
