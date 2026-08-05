use crate::server::{
    auth::middleware::{
        auth::{AuthError, AuthenticatedEntity},
        cache::CachedOrganization,
        permissions::{Authorized, Viewer},
    },
    billing::types::base::{BillingPlan, LimitSource},
    config::AppState,
    organizations::r#impl::base::Organization,
    shared::{
        events::{
            traits::{Event, OrgScope},
            types::BillingOperation,
        },
        types::api::ApiError,
    },
    users::r#impl::permissions::UserOrgPermissions,
};
use async_trait::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};

/// Context available for feature/quota checks
pub struct FeatureCheckContext<'a> {
    pub organization: &'a Organization,
    pub plan: BillingPlan,
    pub app_state: &'a AppState,
    pub permissions: UserOrgPermissions,
}

pub enum FeatureCheckResult {
    Allowed,
    Denied { error: ApiError },
    PaymentRequired { message: String },
}

impl FeatureCheckResult {
    pub fn denied(msg: impl Into<String>) -> Self {
        let message = msg.into();
        Self::Denied {
            error: ApiError::forbidden(&message),
        }
    }

    pub fn denied_with_error(error: ApiError) -> Self {
        Self::Denied { error }
    }

    pub fn payment_required(msg: impl Into<String>) -> Self {
        Self::PaymentRequired {
            message: msg.into(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

#[async_trait]
pub trait FeatureCheck: Send + Sync + Default {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult;
}

// ============ Extractor ============

pub struct RequireFeature<T: FeatureCheck> {
    pub permissions: UserOrgPermissions,
    pub plan: BillingPlan,
    pub organization: Organization,
    pub _phantom: std::marker::PhantomData<T>,
}

impl<S, T> FromRequestParts<S> for RequireFeature<T>
where
    S: Send + Sync + AsRef<AppState>,
    T: FeatureCheck + Default,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = Authorized::<Viewer>::from_request_parts(parts, state).await?;
        let permissions = auth
            .entity
            .permissions()
            .ok_or_else(|| AuthError(ApiError::internal_error("No permissions")))?;
        let organization_id = auth
            .organization_id()
            .ok_or_else(|| AuthError(ApiError::internal_error("No organization")))?;

        let app_state = state.as_ref();

        // Use cached organization lookup (may have been cached by billing middleware)
        let organization = CachedOrganization::get_or_load(parts, app_state, &organization_id)
            .await
            .map_err(AuthError)?;

        // Plan from the cached organization row.
        let plan = organization
            .base
            .plan
            .unwrap_or_else(crate::server::billing::plans::get_free_plan);

        let ctx = FeatureCheckContext {
            organization: &organization,
            plan,
            app_state,
            permissions,
        };

        let checker = T::default();
        match checker.check(&ctx).await {
            FeatureCheckResult::Allowed => Ok(RequireFeature {
                permissions,
                plan,
                organization,
                _phantom: std::marker::PhantomData,
            }),
            FeatureCheckResult::Denied { error } => Err(AuthError(error)),
            FeatureCheckResult::PaymentRequired { message } => {
                Err(AuthError(ApiError::payment_required(&message)))
            }
        }
    }
}

// ============ Concrete Checkers ============

#[derive(Default)]
pub struct InviteUsersFeature;

#[async_trait]
impl FeatureCheck for InviteUsersFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        if !ctx.plan.can_invite_users() {
            return FeatureCheckResult::denied("Your plan does not include inviting users");
        }

        // Seat check happens in the handler where we have access to the request body
        FeatureCheckResult::Allowed
    }
}

#[derive(Default)]
pub struct ApiKeyFeature;

#[async_trait]
impl FeatureCheck for ApiKeyFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        if !ctx.plan.features().api_access {
            return FeatureCheckResult::payment_required("Your plan does not include api access");
        }

        FeatureCheckResult::Allowed
    }
}

#[derive(Default)]
pub struct ShareViewsFeature;

#[async_trait]
impl FeatureCheck for ShareViewsFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        if !ctx.plan.features().share_views {
            return FeatureCheckResult::payment_required(
                "Your plan does not include sharing. Upgrade to share live network diagrams.",
            );
        }

        FeatureCheckResult::Allowed
    }
}

#[derive(Default)]
pub struct TakeSnapshotFeature;

#[async_trait]
impl FeatureCheck for TakeSnapshotFeature {
    async fn check(&self, ctx: &FeatureCheckContext<'_>) -> FeatureCheckResult {
        use crate::server::billing::types::base::LimitType;

        let retention = ctx
            .plan
            .snapshot_retention_days(ctx.app_state.config.snapshot_retention_days_override);
        if retention == 0 {
            let _ = ctx
                .app_state
                .services
                .event_bus
                .publish(Event::new(
                    OrgScope {
                        organization_id: ctx.organization.id,
                    },
                    BillingOperation::FeatureLimitHit {
                        limit_type: LimitType::Snapshots,
                        current_count: 0,
                        limit: 0,
                        plan: ctx.plan,
                        source: LimitSource::Api,
                    },
                    AuthenticatedEntity::System,
                ))
                .await;

            return FeatureCheckResult::payment_required(
                "Snapshots aren't available on your plan. Upgrade to capture point-in-time topology.",
            );
        }

        FeatureCheckResult::Allowed
    }
}
