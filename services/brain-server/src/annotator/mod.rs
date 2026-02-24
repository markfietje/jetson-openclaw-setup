//! Multi-domain annotation engine for brain-rs
//!
//! Extracts entities and relationships from markdown content
//! using domain-specific configuration files.

pub mod domains;
pub mod extractor;

use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

pub use domains::{DomainConfig, KnowledgeDomain};

pub(crate) use extractor::EntityExtractor;

/// Main annotator engine
#[derive(Clone)]
pub struct Annotator {
    domains: Arc<HashMap<KnowledgeDomain, DomainConfig>>,
    enabled: bool,
}

impl Annotator {
    /// Create a new annotator from configuration directory
    pub fn new(config_dir: impl Into<std::path::PathBuf>, enabled: bool) -> Result<Self> {
        let domains = Self::load_domains(config_dir.into())?;
        Ok(Self {
            domains: Arc::new(domains),
            enabled,
        })
    }

    /// Create a disabled annotator (for testing)
    pub fn disabled() -> Self {
        Self {
            domains: Arc::new(HashMap::new()),
            enabled: false,
        }
    }

    /// Annotate content and return extracted entities/relationships
    pub fn annotate(&self, content: &str, title: &str) -> Vec<Annotation> {
        if !self.enabled {
            return Vec::new();
        }

        self.domains
            .iter()
            .flat_map(|(&domain, config)| EntityExtractor::extract(content, title, config, domain))
            .collect()
    }

    /// Get loaded domains count
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Load all domain configurations from directory
    fn load_domains(
        config_dir: impl Into<std::path::PathBuf>,
    ) -> Result<HashMap<KnowledgeDomain, DomainConfig>> {
        let config_dir = config_dir.into();
        const DOMAINS: &[(KnowledgeDomain, &str)] = &[
            (KnowledgeDomain::Business, "business.toml"),
            (KnowledgeDomain::TravelPets, "travel_pets.toml"),
            (KnowledgeDomain::Health, "health.toml"),
            (KnowledgeDomain::Technology, "technology.toml"),
            (KnowledgeDomain::PhilippinesExpat, "philippines_expat.toml"),
        ];

        let mut domains = HashMap::new();

        for &(domain, file) in DOMAINS {
            let path = config_dir.join(file);
            match DomainConfig::from_file(&path) {
                Ok(config) => {
                    println!("✅ Loaded domain: {domain} ({})", path.display());
                    domains.insert(domain, config);
                }
                Err(e) => eprintln!("❌ Failed to load {domain} from {}: {e}", path.display()),
            }
        }

        if domains.is_empty() {
            eprintln!(
                "⚠️  No domain configurations loaded from {}",
                config_dir.display()
            );
        } else {
            println!("📚 Loaded {} domain configurations", domains.len());
        }

        Ok(domains)
    }
}

/// Extracted annotation
#[derive(Debug, Clone, serde::Serialize)]
pub struct Annotation {
    pub relation: String,
    pub entity: String,
    pub domain: KnowledgeDomain,
    pub confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_annotator() {
        let annotator = Annotator::disabled();
        assert!(!annotator.enabled);
        assert_eq!(annotator.domain_count(), 0);
        assert!(annotator.annotate("test", "title").is_empty());
    }
}
