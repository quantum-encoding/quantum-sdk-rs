//! Region-scoped inference routing.
//!
//! The gateway routes inference in-region when a region is attached to the
//! work (EU AI Act Art 50 compliance shipped 2026-08-19): a key minted with
//! a region scope routes every request made with it, and a chat request can
//! override that scope for one call via `provider_options.region`. Regions
//! pick the serving endpoints inside the ONE gateway host
//! (`https://api.quantumencoding.ai`) — there is no region-per-hostname.
//!
//! Two places a region is expressed on the wire, both typed here:
//!
//! - key mint: [`crate::CreateKeyRequest::region`]
//! - per-chat override: [`crate::ChatRequest::region`] (chat only — the
//!   agent endpoint routes by key scope by design)
//!
//! The backend accepts region ALIASES and silently degrades anything it
//! doesn't recognize to unscoped legacy routing — never an error. [`Region::parse`]
//! therefore rejects unknown values client-side instead of letting a typo
//! route silently unscoped.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// An inference region for region-scoped routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    /// US-serving endpoints (Vertex us-rep + us-central1).
    Americas,
    /// EU-serving endpoints (Vertex eu-rep + europe-west4).
    Europe,
    /// Asia-serving endpoints (DashScope token-plan ap-southeast-1 for
    /// qwen3.6+, intl for the long tail).
    Asia,
}

impl Region {
    /// The wire value (`"americas"`, `"europe"`, `"asia"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Region::Americas => "americas",
            Region::Europe => "europe",
            Region::Asia => "asia",
        }
    }

    /// Parses a region, tolerating the aliases the backend accepts
    /// (`us`/`america`, `eu`/`eea`, `apac`/`asia-pacific`),
    /// case-insensitively. Returns `None` for anything else — the backend
    /// would degrade an unknown value to unscoped routing without an error,
    /// so the SDK refuses it instead.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "americas" | "america" | "us" => Some(Region::Americas),
            "europe" | "eu" | "eea" => Some(Region::Europe),
            "asia" | "apac" | "asia-pacific" => Some(Region::Asia),
            _ => None,
        }
    }
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parses the canonical names and the backend's aliases; an unknown value
/// is an error (see [`Region::parse`]).
impl FromStr for Region {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Region::parse(s).ok_or_else(|| {
            format!(
                "unknown region '{s}' — expected americas | europe | asia \
                 (aliases: us, eu, apac)"
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_values_round_trip() {
        for r in [Region::Americas, Region::Europe, Region::Asia] {
            assert_eq!(Region::parse(r.as_str()), Some(r));
            assert_eq!(r.to_string(), r.as_str());
        }
    }

    #[test]
    fn aliases_are_tolerated_case_insensitively() {
        assert_eq!(Region::parse("US"), Some(Region::Americas));
        assert_eq!(Region::parse("america"), Some(Region::Americas));
        assert_eq!(Region::parse("eu"), Some(Region::Europe));
        assert_eq!(Region::parse("EEA"), Some(Region::Europe));
        assert_eq!(Region::parse("apac"), Some(Region::Asia));
        assert_eq!(Region::parse("Asia-Pacific"), Some(Region::Asia));
        assert_eq!(Region::parse("  europe "), Some(Region::Europe));
    }

    #[test]
    fn unknown_regions_are_rejected_not_degraded() {
        assert_eq!(Region::parse("africa"), None);
        assert_eq!(Region::parse(""), None);
        assert!("mars".parse::<Region>().is_err());
    }

    #[test]
    fn serde_uses_the_wire_names() {
        assert_eq!(serde_json::to_string(&Region::Asia).unwrap(), "\"asia\"");
        assert_eq!(
            serde_json::from_str::<Region>("\"europe\"").unwrap(),
            Region::Europe
        );
    }
}
