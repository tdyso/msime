//! Development bootstrap for the reviewed desktop dictionary release.
//! Does not activate a generation or modify an installed input method.
use msime_client_core::resources::{ResourceSet, ResourceStore};
use std::io::Read;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("usage: install_resources <resource-cache-directory>")?;
    let specification: ResourceSet = serde_json::from_str(include_str!(
        "../../../resources/desktop-dictionary.lock.json"
    ))?;
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    // Optional download mirror prefix (for restricted networks): MSIME_MIRROR_PREFIX + artifact.url.
    let mirror_prefix = std::env::var("MSIME_MIRROR_PREFIX").unwrap_or_default();
    let destination = ResourceStore::new(root).install(&specification, |artifact| {
        eprintln!("Verifying {}", artifact.name);
        let url = format!("{mirror_prefix}{}", artifact.url);
        let response = client
            .get(&url)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(std::io::Error::other)?;
        Ok(Box::new(response) as Box<dyn Read>)
    })?;
    println!("{}", destination.display());
    Ok(())
}
