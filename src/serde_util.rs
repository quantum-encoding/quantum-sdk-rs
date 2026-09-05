//! Shared serde helpers.

use serde::{Deserialize, Deserializer};

/// Deserialize an explicit `null` as `Default::default()` (e.g. null → empty
/// Vec). The gateway serialises empty lists as `null`, so every list field
/// needs this alongside `#[serde(default)]`, which only covers a missing
/// field, not a null one.
pub(crate) fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}
