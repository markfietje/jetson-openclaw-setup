//! Domain configuration loading and management
//!
//! Handles loading, parsing, and managing domain-specific
//! annotation configurations from TOML files.

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Domain configuration loaded from TOML file
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DomainConfig {
    /// Domain metadata
    #[serde(default)]
    pub domain: DomainMetadata,

    /// Entity definitions organized by category
    #[serde(default)]
    pub entities: Entities,

    /// Relationship definitions
    #[serde(default)]
    pub relations: Relations,

    /// Regex patterns for complex entity matching
    #[serde(default)]
    #[serde(rename = "patterns")]
    pub regex_patterns: RegexPatterns,

    /// Common aliases and synonyms
    #[serde(default)]
    pub aliases: HashMap<String, Vec<String>>,

    /// High-priority entities that should always be extracted
    #[serde(default)]
    #[serde(rename = "high_priority_entities")]
    pub high_priority: HighPriorityEntities,

    /// Common sequences in this domain
    #[serde(default)]
    #[serde(rename = "common_sequences")]
    pub common_sequences: HashMap<String, String>,

    /// Context-specific rules
    #[serde(default)]
    pub context_rules: HashMap<String, Vec<String>>,

    /// Terms to exclude from extraction
    #[serde(default)]
    pub exclusions: Exclusions,
}

impl DomainConfig {
    /// Load domain configuration from TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read domain config from: {}", path.display()))?;

        let config: DomainConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML from: {}", path.display()))?;

        Ok(config)
    }

    /// Get all entities from all categories as a flat set
    pub fn all_entities(&self) -> HashSet<String> {
        self.entities
            .0
            .values()
            .flatten()
            .map(|e| e.to_lowercase())
            .collect()
    }

    /// Get all relationship patterns as a flat reference
    pub fn all_relations(&self) -> &HashMap<String, Vec<String>> {
        &self.relations.0
    }

    /// Check if an entity should be excluded
    pub fn should_exclude(&self, entity: &str) -> bool {
        let entity_lower = entity.to_lowercase();

        // Check exclusions list
        if let Some(exclusions) = self.exclusions.0.get("exclude") {
            if exclusions.iter().any(|e| e.to_lowercase() == entity_lower) {
                return true;
            }
        }

        // Check if it's a very short word (likely noise)
        if entity.len() <= 2 {
            return true;
        }

        false
    }

    /// Check if an entity is high priority
    pub fn is_high_priority(&self, entity: &str) -> bool {
        self.high_priority
            .critical
            .iter()
            .any(|e| e.to_lowercase() == entity.to_lowercase())
    }
}

/// Domain metadata from [domain] section
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DomainMetadata {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub version: String,

    #[serde(default)]
    pub description: String,
}

/// Entity definitions from [entities.*] sections
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Entities(pub HashMap<String, Vec<String>>);

/// Relationship definitions from [relations.*] sections
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Relations(pub HashMap<String, Vec<String>>);

/// Regex patterns from [patterns] section
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RegexPatterns {
    #[serde(default)]
    pub business_entities: Vec<String>,

    #[serde(default)]
    pub money_patterns: Vec<String>,

    #[serde(default)]
    pub tax_patterns: Vec<String>,

    #[serde(default)]
    pub flight_patterns: Vec<String>,

    #[serde(default)]
    pub time_patterns: Vec<String>,

    #[serde(default)]
    pub measurement_patterns: Vec<String>,

    #[serde(default)]
    pub document_patterns: Vec<String>,

    #[serde(default)]
    pub vitamin_forms: Vec<String>,

    #[serde(default)]
    pub dosage_patterns: Vec<String>,

    #[serde(default)]
    pub ratio_patterns: Vec<String>,

    #[serde(default)]
    pub version_patterns: Vec<String>,

    #[serde(default)]
    pub spec_patterns: Vec<String>,

    #[serde(default)]
    pub code_patterns: Vec<String>,

    #[serde(default)]
    pub file_patterns: Vec<String>,

    #[serde(default)]
    pub visa_patterns: Vec<String>,

    #[serde(default)]
    pub date_patterns: Vec<String>,
}

impl RegexPatterns {
    /// Compile all regex patterns in this domain
    #[allow(dead_code)]
    pub fn compile_all(&self) -> Vec<CompiledPattern> {
        let patterns: Vec<(&str, &Vec<String>)> = vec![
            ("business_entities", &self.business_entities),
            ("money_patterns", &self.money_patterns),
            ("tax_patterns", &self.tax_patterns),
            ("flight_patterns", &self.flight_patterns),
            ("time_patterns", &self.time_patterns),
            ("measurement_patterns", &self.measurement_patterns),
            ("document_patterns", &self.document_patterns),
            ("vitamin_forms", &self.vitamin_forms),
            ("dosage_patterns", &self.dosage_patterns),
            ("ratio_patterns", &self.ratio_patterns),
            ("version_patterns", &self.version_patterns),
            ("spec_patterns", &self.spec_patterns),
            ("code_patterns", &self.code_patterns),
            ("file_patterns", &self.file_patterns),
            ("visa_patterns", &self.visa_patterns),
            ("date_patterns", &self.date_patterns),
        ];

        patterns
            .into_iter()
            .flat_map(|(category, patterns)| {
                patterns.iter().filter_map(|p| {
                    Regex::new(p).ok().map(|regex| CompiledPattern {
                        category: category.to_string(),
                        regex,
                        original: p.clone(),
                    })
                })
            })
            .collect()
    }
}

/// Compiled regex pattern with metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompiledPattern {
    pub category: String,
    pub regex: Regex,
    pub original: String,
}

/// High-priority entities from [high_priority_entities] section
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HighPriorityEntities {
    #[serde(default)]
    pub critical: Vec<String>,
}

/// Exclusions from [exclusions] section
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Exclusions(pub HashMap<String, Vec<String>>);

/// Knowledge domain enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[allow(dead_code)]
pub enum KnowledgeDomain {
    Business,
    TravelPets,
    Health,
    Technology,
    PhilippinesExpat,
    General,
}

impl std::fmt::Display for KnowledgeDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Business => write!(f, "business"),
            Self::TravelPets => write!(f, "travel_pets"),
            Self::Health => write!(f, "health"),
            Self::Technology => write!(f, "technology"),
            Self::PhilippinesExpat => write!(f, "philippines_expat"),
            Self::General => write!(f, "general"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_domain_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");

        let toml_content = r#"
[domain]
name = "Test Domain"
version = "1.0"
description = "Test configuration"

[entities]
test_entities = ["entity1", "entity2", "entity3"]

[relations]
test_relation = ["pattern1", "pattern2"]

[exclusions]
exclude = ["the", "a", "an"]
"#;

        fs::write(&config_path, toml_content).unwrap();

        let config = DomainConfig::from_file(&config_path).unwrap();

        assert_eq!(config.domain.name, "Test Domain");
        assert_eq!(config.domain.version, "1.0");
        assert_eq!(config.entities.0.len(), 1);
        assert_eq!(config.entities.0["test_entities"].len(), 3);
        assert_eq!(config.relations.0.len(), 1);
        assert_eq!(config.exclusions.0["exclude"].len(), 3);
    }

    #[test]
    fn test_all_entities() {
        let mut config = DomainConfig::default();
        config.entities.0.insert(
            "test".to_string(),
            vec!["Entity1".to_string(), "Entity2".to_string()],
        );

        let all = config.all_entities();
        assert_eq!(all.len(), 2);
        assert!(all.contains("entity1"));
        assert!(all.contains("entity2"));
    }

    #[test]
    fn test_should_exclude() {
        let mut config = DomainConfig::default();
        config.exclusions.0.insert(
            "exclude".to_string(),
            vec!["noise".to_string(), "the".to_string()],
        );

        assert!(config.should_exclude("noise"));
        assert!(config.should_exclude("Noise")); // case-insensitive
        assert!(!config.should_exclude("valid_entity"));
    }

    #[test]
    fn test_high_priority() {
        let mut config = DomainConfig::default();
        config.high_priority.critical = vec!["important".to_string(), "urgent".to_string()];

        assert!(config.is_high_priority("important"));
        assert!(config.is_high_priority("Important")); // case-insensitive
        assert!(!config.is_high_priority("normal"));
    }

    #[test]
    fn test_compiled_patterns() {
        let patterns = RegexPatterns {
            version_patterns: vec![r"\b\d+\.\d+\.\d\b".to_string()],
            ..Default::default()
        };

        let compiled = patterns.compile_all();
        assert_eq!(compiled.len(), 1);
        assert_eq!(compiled[0].category, "version_patterns");
        assert!(compiled[0].regex.is_match("1.2.3"));
        assert!(!compiled[0].regex.is_match("abc"));
    }
}
