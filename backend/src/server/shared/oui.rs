use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

const OUI_CSV: &str = include_str!("../../../assets/oui.csv");

#[derive(Debug, Clone, Deserialize)]
struct OuiRecord {
    #[serde(rename = "Registry")]
    _registry: String,
    #[serde(rename = "Assignment")]
    assignment: String,
    #[serde(rename = "Organization Name")]
    organization_name: String,
    #[serde(rename = "Organization Address")]
    _organization_address: String,
}

#[derive(Debug, Clone)]
pub struct OuiEntry {
    pub company_name: String,
}

type OuiDatabase = HashMap<[u8; 3], OuiEntry>;

static OUI_DB: OnceLock<OuiDatabase> = OnceLock::new();

fn parse_oui_prefix(hex: &str) -> Option<[u8; 3]> {
    let hex = hex.trim();
    if hex.len() != 6 {
        return None;
    }
    let bytes = [
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ];
    Some(bytes)
}

fn load_database() -> OuiDatabase {
    let mut db = HashMap::new();
    let mut reader = csv::Reader::from_reader(OUI_CSV.as_bytes());

    for result in reader.deserialize::<OuiRecord>() {
        let Ok(record) = result else {
            continue;
        };
        let Some(prefix) = parse_oui_prefix(&record.assignment) else {
            continue;
        };
        db.insert(
            prefix,
            OuiEntry {
                company_name: record.organization_name,
            },
        );
    }

    db
}

fn get_database() -> &'static OuiDatabase {
    OUI_DB.get_or_init(load_database)
}

/// Parse a MAC address string into its 3-byte OUI prefix.
/// Supports formats: "AA:BB:CC:DD:EE:FF", "AA-BB-CC-DD-EE-FF", "AABBCCDDEEFF", "AA:BB:CC"
fn mac_to_prefix(mac: &str) -> Option<[u8; 3]> {
    let hex: String = mac.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if hex.len() < 6 {
        return None;
    }
    parse_oui_prefix(&hex[..6])
}

/// Look up the vendor for a MAC address.
pub fn lookup_by_mac(mac: &str) -> Option<&'static OuiEntry> {
    let prefix = mac_to_prefix(mac)?;
    get_database().get(&prefix)
}

/// Return the number of entries in the OUI database.
pub fn entry_count() -> usize {
    get_database().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oui_database_loads() {
        let count = entry_count();
        assert!(
            count >= 30_000,
            "Expected at least 30,000 OUI entries, got {count}"
        );
    }

    #[test]
    fn test_known_mac_lookups() {
        // Apple owns many OUI prefixes; 3C:22:FB is one
        let entry = lookup_by_mac("3C:22:FB:00:00:00");
        assert!(entry.is_some(), "Expected to find Apple OUI");
        assert!(
            entry.unwrap().company_name.to_lowercase().contains("apple"),
            "Expected Apple, got: {}",
            entry.unwrap().company_name
        );
    }

    #[test]
    fn test_vendor_constants_in_database() {
        use crate::server::services::r#impl::patterns::Vendor;

        let normalize = |s: &str| -> String {
            s.trim()
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect()
        };

        let db = get_database();

        let vendors = [
            Vendor::PHILIPS,
            Vendor::HP,
            Vendor::EERO,
            Vendor::TPLINK,
            Vendor::UBIQUITI,
            Vendor::GOOGLE,
            Vendor::NEST,
            Vendor::AMAZON,
            Vendor::SONOS,
            Vendor::ECOBEE,
            Vendor::ROKU,
            Vendor::ROBOROCK,
        ];

        for vendor in vendors {
            let normalized_vendor = normalize(vendor);
            let found = db
                .values()
                .any(|entry| normalize(&entry.company_name) == normalized_vendor);
            assert!(
                found,
                "Vendor constant '{}' not found in OUI database",
                vendor
            );
        }
    }

    #[test]
    fn test_unknown_mac_returns_none() {
        // These prefixes may or may not be assigned; just verify no panic
        let _ = lookup_by_mac("00:00:00:00:00:00");
        let _ = lookup_by_mac("FF:FF:FF:FF:FF:FF");
        // Test with too-short input
        assert!(lookup_by_mac("AA:BB").is_none());
        assert!(lookup_by_mac("").is_none());
    }

    #[test]
    fn test_mac_format_handling() {
        // All these formats should parse to the same prefix and return the same result
        let colon = lookup_by_mac("3C:22:FB:00:00:00");
        let dash = lookup_by_mac("3C-22-FB-00-00-00");
        let bare = lookup_by_mac("3C22FB000000");
        let short = lookup_by_mac("3C:22:FB");

        assert!(colon.is_some());
        assert_eq!(
            colon.unwrap().company_name,
            dash.unwrap().company_name,
            "Dash format should match colon format"
        );
        assert_eq!(
            colon.unwrap().company_name,
            bare.unwrap().company_name,
            "Bare hex format should match colon format"
        );
        assert_eq!(
            colon.unwrap().company_name,
            short.unwrap().company_name,
            "Short format should match colon format"
        );
    }
}
