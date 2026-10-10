//! Serde structs for Docker API responses.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VersionInfo {
    pub version: String,
    pub api_version: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_from_fixture() {
        let raw = include_str!("../../tests/fixtures/version.http");
        let body = &raw[raw.find("\r\n\r\n").unwrap() + 4..];
        let v: VersionInfo = serde_json::from_str(body).unwrap();
        assert_eq!(v.version, "29.4.0");
        assert_eq!(v.api_version, "1.54");
    }
}
