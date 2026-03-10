use crate::client::ApiClient;
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiGroupList {
    groups: Vec<ApiGroup>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiGroup {
    name: String,
    versions: Vec<GroupVersionForDiscovery>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupVersionForDiscovery {
    group_version: String,
}

/// Display supported API versions
pub async fn execute(client: &ApiClient) -> Result<()> {
    let mut versions = Vec::new();

    // Add core API version
    versions.push("v1".to_string());

    // Get API groups
    let api_groups: ApiGroupList = client
        .get("/apis")
        .await
        .map_err(|e| match e {
            crate::client::GetError::NotFound => anyhow::anyhow!("API groups not found"),
            crate::client::GetError::Other(e) => e,
        })?;

    for group in api_groups.groups {
        for version in group.versions {
            versions.push(version.group_version);
        }
    }

    // Sort versions
    versions.sort();

    // Print each version
    for version in versions {
        println!("{}", version);
    }

    Ok(())
}
