use sqlx::{PgPool, Row, postgres::PgRow};
use strum::IntoEnumIterator;
use thiserror::Error;
use uuid::Uuid;
use validator::Validate;

use crate::server::{
    known_ports::types::{CatalogueSource, KnownPort, KnownPortInput},
    ports::r#impl::base::{PortType, TransportProtocol},
    shared::types::metadata::{HasId, TypeMetadataProvider},
};

#[derive(Debug, Error)]
pub enum KnownPortServiceError {
    #[error("Custom known port not found")]
    NotFound,
    #[error("A known port already exists for this port and protocol")]
    Conflict,
    #[error("{0}")]
    Validation(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub struct KnownPortService {
    pool: PgPool,
}

impl KnownPortService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The single merge point for the compile-time catalogue and the caller's
    /// organization-owned extensions. Consumers must not merge these layers in
    /// the UI, because resolution and collision rules belong to the backend.
    pub async fn list(
        &self,
        organization_id: Uuid,
    ) -> Result<Vec<KnownPort>, KnownPortServiceError> {
        let rows = sqlx::query(
            "SELECT id, organization_id, name, description, port_number, transport_protocol
             FROM custom_known_ports
             WHERE organization_id = $1",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        let mut ports = Self::built_in_ports();
        ports.extend(
            rows.iter()
                .map(Self::custom_from_row)
                .collect::<Result<Vec<_>, _>>()?,
        );
        ports.sort_by(|left, right| {
            (left.port_number, left.transport_protocol, &left.name).cmp(&(
                right.port_number,
                right.transport_protocol,
                &right.name,
            ))
        });
        Ok(ports)
    }

    pub async fn create(
        &self,
        organization_id: Uuid,
        input: KnownPortInput,
    ) -> Result<KnownPort, KnownPortServiceError> {
        let input = Self::validate(input)?;
        Self::reject_builtin_collision(&input)?;
        let row = sqlx::query(
            "INSERT INTO custom_known_ports (
                id, organization_id, name, description, port_number,
                transport_protocol, created_at, updated_at
             ) VALUES ($1, $2, $3, $4, $5, $6, now(), now())
             RETURNING id, organization_id, name, description, port_number, transport_protocol",
        )
        .bind(Uuid::new_v4())
        .bind(organization_id)
        .bind(input.name)
        .bind(input.description)
        .bind(i64::from(input.port_number))
        .bind(input.transport_protocol.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(Self::map_write_error)?;

        Self::custom_from_row(&row)
    }

    pub async fn update(
        &self,
        organization_id: Uuid,
        id: Uuid,
        input: KnownPortInput,
    ) -> Result<KnownPort, KnownPortServiceError> {
        let input = Self::validate(input)?;
        Self::reject_builtin_collision(&input)?;
        let row = sqlx::query(
            "UPDATE custom_known_ports
             SET name = $3, description = $4, port_number = $5,
                 transport_protocol = $6, updated_at = now()
             WHERE id = $1 AND organization_id = $2
             RETURNING id, organization_id, name, description, port_number, transport_protocol",
        )
        .bind(id)
        .bind(organization_id)
        .bind(input.name)
        .bind(input.description)
        .bind(i64::from(input.port_number))
        .bind(input.transport_protocol.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(Self::map_write_error)?
        .ok_or(KnownPortServiceError::NotFound)?;

        Self::custom_from_row(&row)
    }

    pub async fn delete(
        &self,
        organization_id: Uuid,
        id: Uuid,
    ) -> Result<(), KnownPortServiceError> {
        let result =
            sqlx::query("DELETE FROM custom_known_ports WHERE id = $1 AND organization_id = $2")
                .bind(id)
                .bind(organization_id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(KnownPortServiceError::NotFound);
        }
        Ok(())
    }

    fn built_in_ports() -> Vec<KnownPort> {
        PortType::iter()
            .filter(|port| !port.is_custom())
            .map(|port| KnownPort {
                id: port.id().to_string(),
                organization_id: None,
                source: CatalogueSource::BuiltIn,
                name: port.name().to_string(),
                description: Some(port.description().to_string()),
                port_number: port.number(),
                transport_protocol: port.protocol(),
            })
            .collect()
    }

    fn validate(input: KnownPortInput) -> Result<KnownPortInput, KnownPortServiceError> {
        let input = input.normalized();
        input
            .validate()
            .map_err(|error| KnownPortServiceError::Validation(error.to_string()))?;
        Ok(input)
    }

    fn reject_builtin_collision(input: &KnownPortInput) -> Result<(), KnownPortServiceError> {
        if PortType::iter().any(|port| {
            !port.is_custom()
                && port.number() == input.port_number
                && port.protocol() == input.transport_protocol
        }) {
            return Err(KnownPortServiceError::Conflict);
        }
        Ok(())
    }

    fn map_write_error(error: sqlx::Error) -> KnownPortServiceError {
        if let sqlx::Error::Database(database_error) = &error
            && database_error.is_unique_violation()
        {
            return KnownPortServiceError::Conflict;
        }
        KnownPortServiceError::Database(error)
    }

    fn custom_from_row(row: &PgRow) -> Result<KnownPort, KnownPortServiceError> {
        let transport_protocol = match row.try_get::<String, _>("transport_protocol")?.as_str() {
            "Tcp" => TransportProtocol::Tcp,
            "Udp" => TransportProtocol::Udp,
            other => {
                return Err(KnownPortServiceError::Database(sqlx::Error::Decode(
                    format!("unknown transport protocol {other:?}").into(),
                )));
            }
        };
        let port_number =
            u16::try_from(row.try_get::<i64, _>("port_number")?).map_err(|error| {
                KnownPortServiceError::Database(sqlx::Error::Decode(Box::new(error)))
            })?;
        let id: Uuid = row.try_get("id")?;

        Ok(KnownPort {
            id: id.to_string(),
            organization_id: Some(row.try_get("organization_id")?),
            source: CatalogueSource::Custom,
            name: row.try_get("name")?,
            description: row.try_get("description")?,
            port_number,
            transport_protocol,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_layer_is_complete_and_tagged() {
        let expected = PortType::iter().filter(|port| !port.is_custom()).count();
        let actual = KnownPortService::built_in_ports();

        assert_eq!(actual.len(), expected);
        assert!(actual.iter().all(|port| {
            port.source == CatalogueSource::BuiltIn && port.organization_id.is_none()
        }));
    }

    #[test]
    fn normalization_trims_user_text_and_discards_empty_description() {
        let normalized = KnownPortService::validate(KnownPortInput {
            name: "  Internal dashboard  ".to_string(),
            description: Some("   ".to_string()),
            port_number: 7443,
            transport_protocol: TransportProtocol::Tcp,
        })
        .expect("valid input");

        assert_eq!(normalized.name, "Internal dashboard");
        assert_eq!(normalized.description, None);
    }

    #[test]
    fn normalization_rejects_whitespace_only_name() {
        let error = KnownPortService::validate(KnownPortInput {
            name: "   ".to_string(),
            description: None,
            port_number: 7443,
            transport_protocol: TransportProtocol::Tcp,
        })
        .expect_err("blank name must fail");

        assert!(matches!(error, KnownPortServiceError::Validation(_)));
    }

    #[test]
    fn custom_endpoint_cannot_shadow_builtin_endpoint() {
        let error = KnownPortService::reject_builtin_collision(&KnownPortInput {
            name: "Not really SSH".to_string(),
            description: None,
            port_number: 22,
            transport_protocol: TransportProtocol::Tcp,
        })
        .expect_err("22/tcp is protected by PortType::Ssh");

        assert!(matches!(error, KnownPortServiceError::Conflict));
    }

    #[test]
    fn custom_endpoint_can_reuse_number_with_a_different_protocol() {
        KnownPortService::reject_builtin_collision(&KnownPortInput {
            name: "UDP 22".to_string(),
            description: None,
            port_number: 22,
            transport_protocol: TransportProtocol::Udp,
        })
        .expect("22/udp does not collide with built-in 22/tcp");
    }
}
