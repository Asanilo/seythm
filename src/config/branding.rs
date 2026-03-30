use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BrandingConfig {
    pub product_name: String,
    pub tagline: String,
    pub ascii_logo: String,
    pub startup_hint: String,
    pub footer_hint: String,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            product_name: "Seythm".to_string(),
            tagline: "terminal rhythm alpha".to_string(),
            ascii_logo: concat!(
                " ██████╗ ███████╗██╗   ██╗████████╗██╗  ██╗███╗   ███╗\n",
                "██╔════╝ ██╔════╝╚██╗ ██╔╝╚══██╔══╝██║  ██║████╗ ████║\n",
                "╚█████╗  █████╗   ╚████╔╝    ██║   ███████║██╔████╔██║\n",
                " ╚═══██╗ ██╔══╝    ╚██╔╝     ██║   ██╔══██║██║╚██╔╝██║\n",
                "██████╔╝ ███████╗   ██║      ██║   ██║  ██║██║ ╚═╝ ██║\n",
                "╚═════╝  ╚══════╝   ╚═╝      ╚═╝   ╚═╝  ╚═╝╚═╝     ╚═╝"
            )
            .to_string(),
            startup_hint: "Press any key to skip startup.".to_string(),
            footer_hint: "alpha shell rhythm preview".to_string(),
        }
    }
}

pub fn bundled_branding_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/brand.toml")
}

pub fn load_branding(path: impl AsRef<Path>) -> Result<BrandingConfig, BrandingIoError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| BrandingIoError::Read {
        path: path.display().to_string(),
        source,
    })?;
    toml::from_str(&raw).map_err(BrandingIoError::Parse)
}

pub fn load_bundled_branding() -> BrandingConfig {
    load_branding(bundled_branding_path()).unwrap_or_default()
}

#[derive(Debug, thiserror::Error)]
pub enum BrandingIoError {
    #[error("failed to read branding file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse branding TOML: {0}")]
    Parse(toml::de::Error),
}
