//! Serde structs for Docker API responses.

use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VersionInfo {
    pub version: String,
    pub api_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerSummary {
    pub id: String,
    #[serde(default, deserialize_with = "nullable")]
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: i64,
    #[serde(default, deserialize_with = "nullable")]
    pub ports: Vec<Port>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Port {
    pub private_port: u16,
    pub public_port: Option<u16>,
    #[serde(rename = "Type")]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageSummary {
    pub id: String,
    /// Empty, `null` or `["<none>:<none>"]` for untagged images, depending on the daemon.
    #[serde(default, deserialize_with = "nullable")]
    pub repo_tags: Vec<String>,
    pub size: i64,
    pub created: i64,
}

/// The part of `GET /containers/{id}/json` we need.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInspect {
    pub config: ContainerConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    #[serde(default)]
    pub tty: bool,
}

/// Result of `POST /images/prune`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PruneReport {
    #[serde(default, deserialize_with = "nullable")]
    pub images_deleted: Vec<PruneItem>,
    pub space_reclaimed: i64,
}

/// One entry of a prune report: either an untagged reference or a deleted image layer.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PruneItem {
    pub deleted: Option<String>,
}

/// Treats JSON `null` like a missing field (some daemons send `"Ports": null`).
fn nullable<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the de-chunked body of a raw HTTP fixture.
    pub fn fixture_body(raw: &[u8]) -> Vec<u8> {
        crate::docker::http::tests::parse(raw).unwrap()
    }

    #[test]
    fn version_from_fixture() {
        let body = fixture_body(include_bytes!("../../tests/fixtures/version.http"));
        let v: VersionInfo = serde_json::from_slice(&body).unwrap();
        assert_eq!(v.version, "29.4.0");
        assert_eq!(v.api_version, "1.54");
    }

    #[test]
    fn containers_from_fixture() {
        let body = fixture_body(include_bytes!("../../tests/fixtures/containers.http"));
        let list: Vec<ContainerSummary> = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.len(), 3);
        let web = list
            .iter()
            .find(|c| c.names == ["/kubustack-test-web"])
            .unwrap();
        assert_eq!(web.state, "running");
        assert_eq!(web.image, "nginx:alpine");
        assert_eq!(web.ports.len(), 2); // IPv4 + IPv6
        assert_eq!(web.ports[0].public_port, Some(18080));
        let stopped = list.iter().find(|c| c.state == "exited").unwrap();
        assert!(stopped.ports.is_empty());
        let udp = list
            .iter()
            .find(|c| c.names == ["/kubustack-test-udp"])
            .unwrap();
        assert!(udp
            .ports
            .iter()
            .any(|p| p.public_port.is_none() && p.private_port == 9000));
    }

    #[test]
    fn images_from_fixture() {
        let body = fixture_body(include_bytes!("../../tests/fixtures/images.http"));
        let list: Vec<ImageSummary> = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.len(), 2);
        let multi = list.iter().find(|i| i.repo_tags.len() == 2).unwrap();
        assert_eq!(
            multi.repo_tags,
            ["kubustack-test-img:one", "kubustack-test-img:two"]
        );
        assert!(multi.id.starts_with("sha256:"));
        assert!(multi.size > 0 && multi.created > 0);
    }

    #[test]
    fn untagged_images() {
        let json = r#"[{"Id":"sha256:a","RepoTags":null,"Size":1,"Created":1},
                       {"Id":"sha256:b","RepoTags":[],"Size":1,"Created":1},
                       {"Id":"sha256:c","Size":1,"Created":1}]"#;
        let list: Vec<ImageSummary> = serde_json::from_str(json).unwrap();
        assert!(list.iter().all(|i| i.repo_tags.is_empty()));
    }

    #[test]
    fn prune_report() {
        let json = r#"{"ImagesDeleted":[{"Untagged":"x:1"},{"Deleted":"sha256:a"},{"Deleted":"sha256:b"}],"SpaceReclaimed":1500000}"#;
        let r: PruneReport = serde_json::from_str(json).unwrap();
        assert_eq!(
            r.images_deleted
                .iter()
                .filter(|i| i.deleted.is_some())
                .count(),
            2
        );
        assert_eq!(r.space_reclaimed, 1_500_000);
        let empty: PruneReport =
            serde_json::from_str(r#"{"ImagesDeleted":null,"SpaceReclaimed":0}"#).unwrap();
        assert!(empty.images_deleted.is_empty());
    }

    #[test]
    fn null_collections() {
        let json = r#"[{"Id":"abc","Names":null,"Image":"x","State":"created","Status":"Created","Created":1,"Ports":null}]"#;
        let list: Vec<ContainerSummary> = serde_json::from_str(json).unwrap();
        assert!(list[0].names.is_empty() && list[0].ports.is_empty());
    }
}
