use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumIter, IntoStaticStr};
use utoipa::ToSchema;

use crate::server::shared::types::{
    Color, Icon,
    metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider},
};

/// Coarse OS classification, derived from [`HostOsGroup`] rather than stored
/// separately — collectors and other guidance logic branch on this when they
/// only care "is this Windows at all", not the specific group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostOsFamily {
    Windows,
    Linux,
    Router,
    Switch,
    Other,
}

/// User-assignable (or collector-suggested) OS grouping for a host. Deliberately
/// coarse — a handful of groups, not a catalog of every distro/vendor/version —
/// so collectors can use it as guidance for which extra commands are safe/useful
/// to run, without trying to be an exhaustive OS fingerprint database.
///
/// `Unknown` is a `#[serde(other)]` forward-compat fallback (mirrors
/// `SubnetType`): a value written by a newer binary that this one doesn't
/// recognize degrades to `Unknown` instead of failing to deserialize.
#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    Eq,
    PartialEq,
    Hash,
    EnumIter,
    IntoStaticStr,
    Display,
    Default,
    ToSchema,
)]
pub enum HostOsGroup {
    Windows,
    Linux,
    /// Debian-derived distros (Debian, Ubuntu, and their derivatives).
    LinuxDebian,
    /// Router/firewall appliances (Cisco, HP, Fortinet, Ubiquiti, etc.) —
    /// intentionally vendor-agnostic; see the group's `description()`.
    Router,
    /// Switches (Cisco, HP, TP-Link, UniFi, etc.).
    Switch,
    #[default]
    #[serde(other)]
    Unknown,
}

impl HostOsGroup {
    pub fn family(&self) -> HostOsFamily {
        match self {
            Self::Windows => HostOsFamily::Windows,
            Self::Linux | Self::LinuxDebian => HostOsFamily::Linux,
            Self::Router => HostOsFamily::Router,
            Self::Switch => HostOsFamily::Switch,
            Self::Unknown => HostOsFamily::Other,
        }
    }
}

impl FromStr for HostOsGroup {
    type Err = std::convert::Infallible;

    /// Never fails: unrecognized text (e.g. a group added by a newer binary,
    /// written to a plain TEXT column with no serde tagging to fall back on)
    /// degrades to `Unknown` rather than erroring the whole row read. Slightly
    /// more defensive than `SubnetType::from_str`'s precedent, which does
    /// return `Err` on unrecognized input — justified here because this is a
    /// non-critical guidance tag, not core host identity.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Windows" => Self::Windows,
            "Linux" => Self::Linux,
            "LinuxDebian" => Self::LinuxDebian,
            "Router" => Self::Router,
            "Switch" => Self::Switch,
            _ => Self::Unknown,
        })
    }
}

impl HasId for HostOsGroup {
    fn id(&self) -> &'static str {
        self.into()
    }
}

impl EntityMetadataProvider for HostOsGroup {
    fn color(&self) -> Color {
        match self {
            Self::Windows => Color::Blue,
            Self::Linux | Self::LinuxDebian => Color::Orange,
            Self::Router => Color::Purple,
            Self::Switch => Color::Indigo,
            Self::Unknown => Color::Gray,
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Windows => Icon::Monitor,
            Self::Linux | Self::LinuxDebian => Icon::Terminal,
            Self::Router => Icon::Router,
            Self::Switch => Icon::EthernetPort,
            Self::Unknown => Icon::CircleQuestionMark,
        }
    }
}

impl TypeMetadataProvider for HostOsGroup {
    fn name(&self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::LinuxDebian => "Linux (Debian-based)",
            Self::Router => "Router",
            Self::Switch => "Switch",
            Self::Unknown => "Unknown",
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Windows => "Any Windows desktop or server version.",
            Self::Linux => "Any Linux distribution.",
            Self::LinuxDebian => "Debian, Ubuntu, and other Debian-derived distributions.",
            Self::Router => "Router or firewall appliance (Cisco, HP, Fortinet, Ubiquiti, etc.).",
            Self::Switch => "Network switch (Cisco, HP, TP-Link, UniFi, etc.).",
            Self::Unknown => "Not assigned or not recognized.",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str_round_trips_every_known_variant() {
        for group in [
            HostOsGroup::Windows,
            HostOsGroup::Linux,
            HostOsGroup::LinuxDebian,
            HostOsGroup::Router,
            HostOsGroup::Switch,
        ] {
            let id = group.id();
            assert_eq!(HostOsGroup::from_str(id).unwrap(), group, "id: {id}");
        }
    }

    #[test]
    fn from_str_degrades_unrecognized_text_to_unknown() {
        assert_eq!(
            HostOsGroup::from_str("SomeFutureGroup").unwrap(),
            HostOsGroup::Unknown
        );
    }

    #[test]
    fn family_groups_debian_under_linux() {
        assert_eq!(HostOsGroup::Linux.family(), HostOsFamily::Linux);
        assert_eq!(HostOsGroup::LinuxDebian.family(), HostOsFamily::Linux);
        assert_eq!(HostOsGroup::Windows.family(), HostOsFamily::Windows);
        assert_eq!(HostOsGroup::Unknown.family(), HostOsFamily::Other);
    }
}
