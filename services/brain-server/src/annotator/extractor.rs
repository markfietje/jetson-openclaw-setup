//! Entity and relationship extraction engine
//!
//! Extracts entities and relationships from text using
//! domain-specific configurations and patterns.

use crate::annotator::domains::{DomainConfig, KnowledgeDomain};
use crate::annotator::Annotation;
use std::collections::HashSet;

/// Entity and relationship extractor (zero-sized type)
pub struct EntityExtractor;

impl EntityExtractor {
    /// Extract entities and relationships from content
    pub fn extract(
        content: &str,
        title: &str,
        config: &DomainConfig,
        domain: KnowledgeDomain,
    ) -> Vec<Annotation> {
        let full_text = if title.is_empty() {
            content.to_string()
        } else {
            format!("{title}\n\n{content}")
        };

        let all_entities: HashSet<_> = config.all_entities();
        let found_entities = Self::find_entities_in_text(&full_text, &all_entities, config);
        let relations = Self::extract_relationships(&full_text, &found_entities, config);

        let mut annotations: Vec<_> = relations
            .into_iter()
            .filter(|(entity, _, _)| !config.should_exclude(entity))
            .map(|(entity, relation, confidence)| Annotation {
                relation,
                entity,
                domain,
                confidence,
            })
            .collect();

        for entity in found_entities {
            if config.is_high_priority(&entity) && !annotations.iter().any(|a| a.entity == entity) {
                annotations.push(Annotation {
                    relation: "mentioned".to_string(),
                    entity,
                    domain,
                    confidence: 1.0,
                });
            }
        }

        annotations
    }

    fn find_entities_in_text(
        text: &str,
        domain_entities: &HashSet<String>,
        config: &DomainConfig,
    ) -> Vec<String> {
        let text_lower = text.to_lowercase();

        let mut found: Vec<_> = domain_entities
            .iter()
            .filter(|e| e.len() > 2 && !config.should_exclude(e))
            .filter(|&entity| Self::is_valid_entity_match(&text_lower, entity))
            .cloned()
            .collect();

        found.sort();
        found.dedup();
        found
    }

    fn is_valid_entity_match(text_lower: &str, entity: &str) -> bool {
        text_lower.match_indices(entity).any(|(pos, m)| {
            let before_valid =
                pos == 0 || !text_lower[pos - 1..].starts_with(|c: char| c.is_alphanumeric());
            let after_end = pos + m.len();
            let after_valid = after_end >= text_lower.len()
                || !text_lower[after_end..].starts_with(|c: char| c.is_alphanumeric());
            before_valid && after_valid
        })
    }

    fn extract_relationships(
        text: &str,
        entities: &[String],
        config: &DomainConfig,
    ) -> Vec<(String, String, f32)> {
        let text_lower = text.to_lowercase();
        let all_relations = config.all_relations();
        let mut relations = Vec::new();

        for entity in entities {
            for (relation_type, patterns) in all_relations {
                for pattern in patterns {
                    let pattern_lower = pattern.to_lowercase();
                    let search_forward = format!("{entity} {pattern_lower}");
                    let search_backward = format!("{pattern_lower} {entity}");

                    if let Some(pos) = text_lower.find(&search_forward) {
                        let after_start = pos + search_forward.len();
                        if let Some(target) =
                            Self::extract_target_after(&text_lower, after_start, entities)
                        {
                            relations.push((
                                entity.clone(),
                                relation_type.clone(),
                                Self::calculate_confidence(&text_lower, entity, pattern, &target),
                            ));
                        }
                    }

                    if let Some(pos) = text_lower.find(&search_backward) {
                        if let Some(source) =
                            Self::extract_target_before(&text_lower, pos, entities)
                        {
                            let confidence =
                                Self::calculate_confidence(&text_lower, &source, pattern, entity);
                            relations.push((source, relation_type.clone(), confidence));
                        }
                    }
                }
            }
        }

        relations
    }

    fn extract_target_after(
        text_lower: &str,
        start_pos: usize,
        entities: &[String],
    ) -> Option<String> {
        let after_text = text_lower
            .get(start_pos..start_pos + 100.min(text_lower.len().saturating_sub(start_pos)))?;
        entities
            .iter()
            .find(|e| after_text.contains(e.as_str()))
            .cloned()
    }

    fn extract_target_before(
        text_lower: &str,
        end_pos: usize,
        entities: &[String],
    ) -> Option<String> {
        let start_pos = end_pos.saturating_sub(100);
        let before_text = text_lower.get(start_pos..end_pos)?;
        entities
            .iter()
            .rfind(|e| before_text.contains(e.as_str()))
            .cloned()
    }

    fn calculate_confidence(text_lower: &str, entity1: &str, relation: &str, entity2: &str) -> f32 {
        let mut confidence: f32 = 0.5;

        let pattern = format!("{entity1} {relation} {entity2}");
        if text_lower.contains(&pattern) {
            confidence += 0.3;
        }

        if text_lower
            .split('.')
            .any(|s| s.contains(entity1) && s.contains(entity2))
        {
            confidence += 0.1;
        }

        if ["is", "are", "was", "were", "has", "have"]
            .iter()
            .any(|p| relation.contains(p))
        {
            confidence += 0.1;
        }

        confidence.min(1.0)
    }
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotator::domains::{DomainConfig, Entities, Relations};
    use std::collections::HashMap;

    fn create_test_config() -> DomainConfig {
        let mut entities = HashMap::new();
        entities.insert(
            "test".to_string(),
            vec!["entity1".to_string(), "entity2".to_string()],
        );

        let mut relations = HashMap::new();
        relations.insert("test_rel".to_string(), vec!["helps".to_string()]);

        let mut exclusions = HashMap::new();
        exclusions.insert("exclude".to_string(), vec!["the".to_string()]);

        DomainConfig {
            entities: Entities(entities),
            relations: Relations(relations),
            exclusions: crate::annotator::domains::Exclusions(exclusions),
            ..Default::default()
        }
    }

    #[test]
    fn test_find_entities() {
        let config = create_test_config();
        let text = "This document discusses entity1 and its benefits.";
        let entities =
            EntityExtractor::find_entities_in_text(text, &config.all_entities(), &config);
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0], "entity1");
    }

    #[test]
    fn test_valid_entity_match() {
        assert!(EntityExtractor::is_valid_entity_match(
            "this is entity1 here",
            "entity1"
        ));
        assert!(!EntityExtractor::is_valid_entity_match(
            "this is entity123",
            "entity1"
        ));
        assert!(EntityExtractor::is_valid_entity_match(
            "entity1 is important",
            "entity1"
        ));
        assert!(EntityExtractor::is_valid_entity_match(
            "this is entity1",
            "entity1"
        ));
    }

    #[test]
    fn test_extract_relationships() {
        let config = create_test_config();
        let text = "entity1 helps entity2 to improve performance";
        let entities = vec!["entity1".to_string(), "entity2".to_string()];
        let relations = EntityExtractor::extract_relationships(text, &entities, &config);
        assert!(!relations.is_empty());
        assert_eq!(relations[0].0, "entity1");
        assert_eq!(relations[0].1, "test_rel");
    }

    #[test]
    fn test_confidence_calculation() {
        let text_lower = "entity1 helps entity2";
        let confidence =
            EntityExtractor::calculate_confidence(text_lower, "entity1", "helps", "entity2");
        assert!(confidence > 0.5);
        assert!(confidence <= 1.0);
    }
}
