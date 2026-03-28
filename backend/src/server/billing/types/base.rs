use crate::server::{
    billing::types::features::Feature,
    shared::types::{
        Color, Icon,
        metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::hash::Hash;
use stripe_product::price::CreatePriceRecurringInterval;
use strum::{Display, EnumDiscriminants, EnumIter, IntoDiscriminant, IntoStaticStr};
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Display,
    IntoStaticStr,
    EnumIter,
    EnumDiscriminants,
    Eq,
    ToSchema,
)]
#[strum_discriminants(derive(IntoStaticStr, Serialize))]
#[serde(tag = "type")]
pub enum BillingPlan {
    Community(PlanConfig),
    Free(PlanConfig),
    Starter(PlanConfig),
    Pro(PlanConfig),
    Team(PlanConfig),
    Business(PlanConfig),
    Enterprise(PlanConfig),
    Demo(PlanConfig),
    CommercialSelfHosted(PlanConfig),
}

impl PartialOrd for BillingPlanDiscriminants {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn cloud_tier(d: &BillingPlanDiscriminants) -> Option<u8> {
            match d {
                BillingPlanDiscriminants::Free => Some(0),
                BillingPlanDiscriminants::Starter => Some(1),
                BillingPlanDiscriminants::Pro => Some(2),
                BillingPlanDiscriminants::Team => Some(3),
                BillingPlanDiscriminants::Business => Some(4),
                BillingPlanDiscriminants::Enterprise => Some(5),
                _ => None,
            }
        }
        match (cloud_tier(self), cloud_tier(other)) {
            (Some(a), Some(b)) => Some(a.cmp(&b)),
            _ if self == other => Some(std::cmp::Ordering::Equal),
            _ => None,
        }
    }
}

impl PartialEq for BillingPlan {
    fn eq(&self, other: &Self) -> bool {
        self.config() == other.config()
    }
}

impl Hash for BillingPlan {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.config().hash(state);
    }
}

impl Default for BillingPlan {
    fn default() -> Self {
        #[cfg(feature = "commercial")]
        {
            use crate::server::billing::plans::get_commercial_self_hosted_plan;

            get_commercial_self_hosted_plan()
        }
        #[cfg(not(feature = "commercial"))]
        {
            use crate::server::billing::plans::get_community_plan;

            get_community_plan()
        }
    }
}

impl BillingPlan {
    pub fn to_yearly(&self, discount: f32) -> Self {
        let mut yearly_config = self.config();
        yearly_config.rate = BillingRate::Year;

        // Round discounted monthly base to nearest dollar then subtract 1 cent
        // so yearly prices end in .99 (e.g. $14.99/mo → $11.99/mo billed yearly).
        let monthly_base = Self::round_to_99(yearly_config.base_cents as f32 * (1.0 - discount));
        yearly_config.base_cents = monthly_base * 12;
        yearly_config.seat_cents = yearly_config.seat_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });
        yearly_config.network_cents = yearly_config.network_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });
        yearly_config.host_cents = yearly_config.host_cents.map(|c| {
            let monthly = Self::round_to_dollar(c as f32 * (1.0 - discount));
            monthly * 12
        });

        let mut yearly_plan = *self;
        yearly_plan.set_config(yearly_config);
        yearly_plan
    }
    fn round_to_dollar(cents: f32) -> i64 {
        ((cents / 100.0).round() * 100.0) as i64
    }

    /// Round to nearest dollar, then subtract 1 cent so the price ends in .99.
    fn round_to_99(cents: f32) -> i64 {
        Self::round_to_dollar(cents) - 1
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Default, Hash, ToSchema)]
pub struct PlanConfig {
    pub base_cents: i64,
    pub rate: BillingRate,
    pub trial_days: u32,

    // None = can't pay for more
    pub seat_cents: Option<i64>,
    pub network_cents: Option<i64>,
    pub host_cents: Option<i64>,

    // None = unlimited
    pub included_seats: Option<u64>,
    pub included_networks: Option<u64>,
    pub included_hosts: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Display, Copy, PartialEq, Eq, Default, Hash)]
pub enum Hosting {
    SelfHosted,
    Managed,
    #[default]
    Cloud,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, Display, Default, Copy, PartialEq, Eq, Hash, ToSchema,
)]
pub enum BillingRate {
    #[default]
    Month,
    Year,
}

impl BillingRate {
    pub fn stripe_recurring_interval(&self) -> CreatePriceRecurringInterval {
        match self {
            BillingRate::Month => CreatePriceRecurringInterval::Month,
            BillingRate::Year => CreatePriceRecurringInterval::Year,
        }
    }

    pub fn billing_period(&self) -> &'static str {
        match self {
            BillingRate::Month => "Monthly",
            BillingRate::Year => "Yearly",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingPlanFeatures {
    pub share_views: bool,
    pub remove_created_with: bool,
    pub audit_logs: bool,
    pub webhooks: bool,
    pub api_access: bool,
    pub onboarding_call: bool,
    pub custom_sso: bool,
    pub managed_deployment: bool,
    pub whitelabeling: bool,
    pub live_chat_support: bool,
    pub embeds: bool,
    pub email_support: bool,
    pub community_support: bool,
    pub priority_support: bool,
    // Core features
    pub network_discovery: bool,
    pub topology_visualization: bool,
    pub png_export: bool,
    pub svg_export: bool,
    pub mermaid_export: bool,
    pub confluence_export: bool,
    pub pdf_export: bool,
    pub html_export: bool,
    pub scheduled_discovery: bool,
    pub daemon_poll: bool,
    pub service_definitions: bool,
    pub docker_integration: bool,
    pub snmp_integration: bool,
    pub csv_export: bool,
}

impl BillingPlan {
    pub fn config(&self) -> PlanConfig {
        match self {
            BillingPlan::Community(plan_config) => *plan_config,
            BillingPlan::Free(plan_config) => *plan_config,
            BillingPlan::Starter(plan_config) => *plan_config,
            BillingPlan::Pro(plan_config) => *plan_config,
            BillingPlan::Team(plan_config) => *plan_config,
            BillingPlan::Business(plan_config) => *plan_config,
            BillingPlan::Enterprise(plan_config) => *plan_config,
            BillingPlan::Demo(plan_config) => *plan_config,
            BillingPlan::CommercialSelfHosted(plan_config) => *plan_config,
        }
    }

    pub fn set_config(&mut self, config: PlanConfig) {
        match self {
            BillingPlan::Community(plan_config) => *plan_config = config,
            BillingPlan::Free(plan_config) => *plan_config = config,
            BillingPlan::Starter(plan_config) => *plan_config = config,
            BillingPlan::Pro(plan_config) => *plan_config = config,
            BillingPlan::Team(plan_config) => *plan_config = config,
            BillingPlan::Business(plan_config) => *plan_config = config,
            BillingPlan::Enterprise(plan_config) => *plan_config = config,
            BillingPlan::Demo(plan_config) => *plan_config = config,
            BillingPlan::CommercialSelfHosted(plan_config) => *plan_config = config,
        }
    }

    pub fn is_commercial(&self) -> bool {
        matches!(
            self,
            BillingPlan::Pro(_)
                | BillingPlan::Team(_)
                | BillingPlan::Business(_)
                | BillingPlan::Enterprise(_)
                | BillingPlan::CommercialSelfHosted(_)
                | BillingPlan::Demo(_)
        )
    }

    pub fn is_free(&self) -> bool {
        matches!(self, BillingPlan::Free(_))
    }

    pub fn is_demo(&self) -> bool {
        matches!(self, BillingPlan::Demo(_))
    }

    pub fn host_limit(&self) -> Option<u64> {
        self.config().included_hosts
    }

    pub fn network_limit(&self) -> Option<u64> {
        self.config().included_networks
    }

    pub fn seat_limit(&self) -> Option<u64> {
        self.config().included_seats
    }

    pub fn can_invite_users(&self) -> bool {
        // If there's an included amount, then there's a cap and seat_cents needs to be Some to buy more
        if self.config().included_seats.is_some() {
            self.config().seat_cents.is_some()
        // If included is None, it's unlimited
        } else {
            true
        }
    }

    pub fn hosting(&self) -> Hosting {
        match self {
            BillingPlan::Community(_) => Hosting::SelfHosted,
            BillingPlan::CommercialSelfHosted(_) => Hosting::SelfHosted,
            BillingPlan::Enterprise(_) => Hosting::Managed,
            _ => Hosting::Cloud, // Free, Starter, Pro, Team, Business, Demo
        }
    }

    /// Returns the next-lower-tier cloud plan, if this is a cloud plan.
    /// Returns None for Free (no previous) and self-hosted/demo plans.
    pub fn previous_tier(&self) -> Option<BillingPlanDiscriminants> {
        let cloud_tiers: Vec<BillingPlanDiscriminants> = vec![
            BillingPlanDiscriminants::Free,
            BillingPlanDiscriminants::Starter,
            BillingPlanDiscriminants::Pro,
            BillingPlanDiscriminants::Business,
            BillingPlanDiscriminants::Enterprise,
        ];

        let my_disc = self.discriminant();
        let idx = cloud_tiers.iter().position(|d| *d == my_disc)?;
        if idx == 0 {
            return None;
        }
        Some(cloud_tiers[idx - 1])
    }

    /// Returns feature IDs added by this plan over its previous tier.
    /// For Free: returns all enabled features (it's the baseline).
    /// For self-hosted plans with no previous tier: returns all enabled non-universal features.
    /// For cloud plans: returns features new vs the previous tier.
    pub fn incremental_features(&self) -> Vec<&'static str> {
        let enabled = self.enabled_feature_ids();

        match self.previous_tier() {
            Some(prev_disc) => {
                let prev_plan = Self::default_for_discriminant(prev_disc);
                match prev_plan {
                    Some(plan) => {
                        let prev_features = plan.enabled_feature_ids();
                        enabled.difference(&prev_features).copied().collect()
                    }
                    None => enabled.into_iter().collect(),
                }
            }
            None if self.is_free() => {
                // Free plan: show all enabled features (it's the baseline)
                enabled.into_iter().collect()
            }
            None => {
                // Self-hosted/other plans: show features beyond universal (Free) baseline
                let universal = Self::universal_feature_ids();
                enabled.difference(&universal).copied().collect()
            }
        }
    }

    /// Returns set of feature IDs where the feature is enabled on this plan.
    pub fn enabled_feature_ids(&self) -> HashSet<&'static str> {
        let features = self.features();
        let json = serde_json::to_value(&features).unwrap();
        let obj = json.as_object().unwrap();
        obj.iter()
            .filter(|(_, v)| v.as_bool().unwrap_or(false))
            .map(|(k, _)| {
                // Leak the key string so we get &'static str
                // This is fine since these are a small fixed set called infrequently
                let s: &'static str = Box::leak(k.clone().into_boxed_str());
                s
            })
            .collect()
    }

    /// Features that are universal across all plans (present on Free).
    fn universal_feature_ids() -> HashSet<&'static str> {
        use crate::server::billing::plans::get_free_plan;
        get_free_plan().enabled_feature_ids()
    }

    /// Whether the feature identified by `feature_id` is enabled on this plan.
    pub fn has_feature(&self, feature_id: &str) -> bool {
        let features = self.features();
        let json = serde_json::to_value(&features).unwrap();
        json.get(feature_id)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Build a default plan instance for a given discriminant (monthly, default config).
    pub fn default_for_discriminant(disc: BillingPlanDiscriminants) -> Option<BillingPlan> {
        use crate::server::billing::plans::*;

        match disc {
            BillingPlanDiscriminants::Free => Some(get_free_plan()),
            BillingPlanDiscriminants::Community => Some(get_community_plan()),
            BillingPlanDiscriminants::Enterprise => Some(get_enterprise_plan()),
            BillingPlanDiscriminants::CommercialSelfHosted => {
                Some(get_commercial_self_hosted_plan())
            }
            // For purchasable plans, find them from the default list
            _ => get_purchasable_plans()
                .into_iter()
                .find(|p| p.discriminant() == disc),
        }
    }

    pub fn custom_price(&self) -> Option<&str> {
        match self {
            BillingPlan::Enterprise(_) => Some("Custom"),
            BillingPlan::Community(_) | BillingPlan::Free(_) => Some("Free"),
            BillingPlan::CommercialSelfHosted(_) => Some("Custom"),
            _ => None,
        }
    }

    pub fn stripe_product_id(&self) -> String {
        self.to_string().to_lowercase()
    }

    pub fn stripe_base_price_lookup_key(&self) -> String {
        format!(
            "{}_{}_{}",
            self.stripe_product_id(),
            self.config().base_cents,
            self.config().rate
        )
    }

    pub fn stripe_seat_addon_price_lookup_key(&self) -> Option<String> {
        self.config().seat_cents.map(|c| {
            format!(
                "{}_seats_{}_{}",
                self.stripe_product_id(),
                c,
                self.config().rate
            )
        })
    }

    pub fn stripe_network_addon_price_lookup_key(&self) -> Option<String> {
        self.config().network_cents.map(|c| {
            format!(
                "{}_networks_{}_{}",
                self.stripe_product_id(),
                c,
                self.config().rate
            )
        })
    }

    pub fn features(&self) -> BillingPlanFeatures {
        match self {
            BillingPlan::Community { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: false,
                api_access: true,
                custom_sso: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: false,
                community_support: true,
                priority_support: false,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: false,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Free { .. } => BillingPlanFeatures {
                share_views: false,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: false,
                custom_sso: false,
                api_access: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: false,
                email_support: false,
                community_support: true,
                priority_support: false,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: false,
                mermaid_export: false,
                confluence_export: false,
                pdf_export: false,
                html_export: false,
                scheduled_discovery: false,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Starter { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                custom_sso: false,
                api_access: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: false,
                email_support: true,
                community_support: true,
                priority_support: false,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: false,
                confluence_export: false,
                pdf_export: false,
                html_export: false,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Pro { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: false,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                api_access: true,
                custom_sso: false,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                community_support: true,
                priority_support: false,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: false,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Team { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: false,
                audit_logs: false,
                remove_created_with: true,
                custom_sso: false,
                api_access: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                community_support: true,
                priority_support: true,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Business { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: false,
                api_access: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                community_support: true,
                priority_support: true,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Enterprise { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: true,
                api_access: true,
                managed_deployment: true,
                whitelabeling: true,
                live_chat_support: true,
                embeds: true,
                email_support: true,
                community_support: true,
                priority_support: true,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::Demo { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                custom_sso: true,
                api_access: true,
                managed_deployment: true,
                whitelabeling: true,
                live_chat_support: true,
                embeds: true,
                email_support: true,
                community_support: true,
                priority_support: true,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
            BillingPlan::CommercialSelfHosted { .. } => BillingPlanFeatures {
                share_views: true,
                onboarding_call: true,
                webhooks: true,
                audit_logs: true,
                remove_created_with: true,
                api_access: true,
                custom_sso: true,
                managed_deployment: false,
                whitelabeling: false,
                live_chat_support: false,
                embeds: true,
                email_support: true,
                community_support: true,
                priority_support: true,
                network_discovery: true,
                topology_visualization: true,
                png_export: true,
                svg_export: true,
                mermaid_export: true,
                confluence_export: true,
                pdf_export: true,
                html_export: true,
                scheduled_discovery: true,
                daemon_poll: true,
                service_definitions: true,
                docker_integration: true,
                snmp_integration: true,
                csv_export: true,
            },
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<Feature>> for BillingPlanFeatures {
    fn into(self) -> Vec<Feature> {
        let mut features = vec![];

        let BillingPlanFeatures {
            share_views,
            onboarding_call,
            webhooks,
            audit_logs,
            remove_created_with,
            custom_sso,
            managed_deployment,
            whitelabeling,
            api_access,
            live_chat_support,
            embeds,
            email_support,
            priority_support,
            community_support,
            network_discovery,
            topology_visualization,
            png_export,
            svg_export,
            mermaid_export,
            confluence_export,
            pdf_export,
            html_export,
            scheduled_discovery,
            daemon_poll,
            service_definitions,
            docker_integration,
            snmp_integration,
            csv_export,
        } = self;

        if share_views {
            features.push(Feature::ShareViews)
        }

        if custom_sso {
            features.push(Feature::CustomSso)
        }

        if api_access {
            features.push(Feature::ApiAccess)
        }

        if managed_deployment {
            features.push(Feature::ManagedDeployment)
        }

        if embeds {
            features.push(Feature::Embeds)
        }

        if whitelabeling {
            features.push(Feature::Whitelabeling)
        }

        if live_chat_support {
            features.push(Feature::LiveChatSupport)
        }

        if priority_support {
            features.push(Feature::PrioritySupport)
        }

        if community_support {
            features.push(Feature::CommunitySupport)
        }

        if email_support {
            features.push(Feature::EmailSupport)
        }

        if onboarding_call {
            features.push(Feature::OnboardingCall)
        }

        if webhooks {
            features.push(Feature::Webhooks);
        }

        if audit_logs {
            features.push(Feature::AuditLogs)
        }

        if remove_created_with {
            features.push(Feature::RemoveCreatedWith)
        }

        if network_discovery {
            features.push(Feature::NetworkDiscovery)
        }

        if topology_visualization {
            features.push(Feature::TopologyVisualization)
        }

        if png_export {
            features.push(Feature::PngExport)
        }

        if svg_export {
            features.push(Feature::SvgExport)
        }

        if mermaid_export {
            features.push(Feature::MermaidExport)
        }

        if confluence_export {
            features.push(Feature::ConfluenceExport)
        }

        if pdf_export {
            features.push(Feature::PdfExport)
        }

        if html_export {
            features.push(Feature::HtmlExport)
        }

        if scheduled_discovery {
            features.push(Feature::ScheduledDiscovery)
        }

        if daemon_poll {
            features.push(Feature::DaemonPoll)
        }

        if service_definitions {
            features.push(Feature::ServiceDefinitions)
        }

        if docker_integration {
            features.push(Feature::DockerDiscovery)
        }

        if snmp_integration {
            features.push(Feature::SnmpDiscovery)
        }

        if csv_export {
            features.push(Feature::CsvExport)
        }

        features
    }
}

impl HasId for BillingPlan {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for BillingPlan {
    fn icon(&self) -> Icon {
        match self {
            BillingPlan::Community { .. } => Icon::Heart,
            BillingPlan::Free { .. } => Icon::Gift,
            BillingPlan::Starter { .. } => Icon::ThumbsUp,
            BillingPlan::Pro { .. } => Icon::Zap,
            BillingPlan::Team { .. } => Icon::Users,
            BillingPlan::Business { .. } => Icon::Briefcase,
            BillingPlan::Enterprise { .. } => Icon::Building,
            BillingPlan::Demo { .. } => Icon::TestTube,
            BillingPlan::CommercialSelfHosted { .. } => Icon::ServerCog,
        }
    }

    fn color(&self) -> Color {
        match self {
            BillingPlan::Community { .. } => Color::Pink,
            BillingPlan::Free { .. } => Color::Green,
            BillingPlan::Starter { .. } => Color::Blue,
            BillingPlan::Pro { .. } => Color::Yellow,
            BillingPlan::Team { .. } => Color::Orange,
            BillingPlan::Business { .. } => Color::Indigo,
            BillingPlan::Enterprise { .. } => Color::Teal,
            BillingPlan::Demo { .. } => Color::Purple,
            BillingPlan::CommercialSelfHosted { .. } => Color::Gray,
        }
    }
}

impl TypeMetadataProvider for BillingPlan {
    fn name(&self) -> &'static str {
        match self {
            BillingPlan::Community { .. } => "Community",
            BillingPlan::Free { .. } => "Free",
            BillingPlan::Starter { .. } => "Starter",
            BillingPlan::Pro { .. } => "Pro",
            BillingPlan::Team { .. } => "Team",
            BillingPlan::Business { .. } => "Business",
            BillingPlan::Enterprise { .. } => "Enterprise",
            BillingPlan::Demo { .. } => "Demo",
            BillingPlan::CommercialSelfHosted { .. } => "On-Premise",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            BillingPlan::Community { .. } => {
                "Community plan for individuals self-hosting Scanopy - full control over configuration and integrations"
            }
            BillingPlan::Free { .. } => {
                "Explore your network: discover and document up to 25 hosts"
            }
            BillingPlan::Starter { .. } => {
                "Living network documentation that keeps itself up to date"
            }
            BillingPlan::Pro { .. } => {
                "For professionals managing and monitoring multiple networks"
            }
            BillingPlan::Team { .. } => {
                "Collaborate on infrastructure documentation with your team"
            }
            BillingPlan::Business { .. } => {
                "For MSPs and multi-site IT teams who need advanced features"
            }
            BillingPlan::Enterprise { .. } => {
                "Fully managed Scanopy deployment with dedicated support"
            }
            BillingPlan::Demo { .. } => "Demo mode",
            BillingPlan::CommercialSelfHosted { .. } => {
                "Commercial license for self-managed deployments — full control over configuration and integrations"
            }
        }
    }

    fn metadata(&self) -> serde_json::Value {
        let config = self.config();
        let previous_tier = self
            .previous_tier()
            .and_then(BillingPlan::default_for_discriminant)
            .map(|p| p.id());

        serde_json::json!({
            // Pricing information
            "base_cents": config.base_cents,
            "rate": config.rate,
            "trial_days": config.trial_days,
            "seat_cents": config.seat_cents,
            "network_cents": config.network_cents,
            "host_cents": config.host_cents,
            "included_seats": config.included_seats,
            "included_networks": config.included_networks,
            "included_hosts": config.included_hosts,
            // Feature flags and metadata
            "features": self.features(),
            "is_commercial": self.is_commercial(),
            "hosting": self.hosting(),
            "custom_price": self.custom_price(),
            // Tier relationship
            "incremental_features": self.incremental_features(),
            "previous_tier": previous_tier
        })
    }
}
