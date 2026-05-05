//! Skill RAG (Retrieval-Augmented Generation) — discover skills by text search or embedding similarity.

use crate::SkillRegistry;

/// A scored skill search result.
#[derive(Debug, Clone)]
pub struct ScoredSkill {
    pub name: String,
    pub description: String,
    pub score: f64,
    pub match_reason: String,
}

/// Search skills by text: exact tag match, then name/description substring, then token overlap.
pub fn search_by_text(registry: &SkillRegistry, query: &str, limit: usize) -> Vec<ScoredSkill> {
    let query_lower = query.to_lowercase();
    let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();
    let mut results: Vec<ScoredSkill> = Vec::new();

    for skill in registry.all() {
        let mut score = 0.0;
        let mut reasons: Vec<&str> = Vec::new();

        // Tier 1: Exact tag match (highest weight)
        for tag in &skill.tags {
            let tag_lower = tag.to_lowercase();
            if query_tokens.contains(&tag_lower.as_str()) {
                score += 10.0;
                reasons.push("tag match");
            }
        }

        // Tier 2: Name or description substring match
        let name_lower = skill.name.to_lowercase();
        let desc_lower = skill.description.to_lowercase();
        if name_lower.contains(&query_lower) || query_lower.contains(&name_lower) {
            score += 5.0;
            reasons.push("name match");
        }
        if desc_lower.contains(&query_lower) {
            score += 3.0;
            reasons.push("description match");
        }

        // Tier 3: Token overlap
        let skill_tokens: Vec<String> = skill.tags.iter()
            .chain(std::iter::once(&skill.name))
            .chain(std::iter::once(&skill.description))
            .flat_map(|s| s.to_lowercase().split_whitespace().map(String::from).collect::<Vec<_>>())
            .collect();
        let overlap: usize = query_tokens.iter()
            .filter(|qt| skill_tokens.iter().any(|st| st.contains(*qt) || qt.contains(st.as_str())))
            .count();
        if overlap > 0 {
            score += overlap as f64 * 1.0;
            if !reasons.contains(&"tag match") && !reasons.contains(&"name match") {
                reasons.push("token overlap");
            }
        }

        if score > 0.0 {
            results.push(ScoredSkill {
                name: skill.name.clone(),
                description: skill.description.clone(),
                score,
                match_reason: reasons.join(", "),
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

/// Search skills by tags — returns all skills that have any of the given tags.
pub fn search_by_tags(registry: &SkillRegistry, tags: &[String], limit: usize) -> Vec<ScoredSkill> {
    let mut results: Vec<ScoredSkill> = Vec::new();

    for skill in registry.all() {
        let matched_tags: Vec<&String> = skill.tags.iter()
            .filter(|t| tags.contains(t))
            .collect();
        if !matched_tags.is_empty() {
            results.push(ScoredSkill {
                name: skill.name.clone(),
                description: skill.description.clone(),
                score: matched_tags.len() as f64 * 10.0,
                match_reason: format!("tags: {}", matched_tags.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ")),
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

/// Cosine similarity between two f32 vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Search skills by embedding similarity against pre-stored vectors.
/// `query_embedding` is the embedded query, `stored` is (skill_name, embedding_bytes).
/// Embedding bytes are stored as packed f32 little-endian.
pub fn search_by_embedding(
    query_embedding: &[f32],
    stored: &[(String, Vec<u8>)],
    registry: &SkillRegistry,
    limit: usize,
) -> Vec<ScoredSkill> {
    let mut results: Vec<ScoredSkill> = Vec::new();

    for (name, bytes) in stored {
        let embedding = bytes_to_f32(bytes);
        let sim = cosine_similarity(query_embedding, &embedding);
        if sim > 0.3 {
            let description = registry.get(name)
                .map(|s| s.description.clone())
                .unwrap_or_default();
            results.push(ScoredSkill {
                name: name.clone(),
                description,
                score: sim as f64,
                match_reason: format!("embedding similarity: {:.3}", sim),
            });
        }
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

fn bytes_to_f32(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Skill;
    use std::path::PathBuf;

    fn make_registry() -> SkillRegistry {
        let mut skills = std::collections::HashMap::new();
        skills.insert("cloudflare-dns".to_string(), Skill {
            name: "cloudflare-dns".to_string(),
            description: "Manage Cloudflare DNS records".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["infrastructure".to_string(), "dns".to_string(), "cloudflare".to_string()],
            requires_tools: vec!["web_fetch".to_string()],
            content: String::new(),
            file_path: PathBuf::new(),
        });
        skills.insert("github".to_string(), Skill {
            name: "github".to_string(),
            description: "GitHub operations via CLI".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["development".to_string(), "github".to_string(), "git".to_string()],
            requires_tools: vec!["bash".to_string()],
            content: String::new(),
            file_path: PathBuf::new(),
        });
        skills.insert("debugging".to_string(), Skill {
            name: "debugging".to_string(),
            description: "Systematic debugging methodology".to_string(),
            version: "1.0.0".to_string(),
            tags: vec!["methodology".to_string(), "development".to_string()],
            requires_tools: vec!["read_file".to_string()],
            content: String::new(),
            file_path: PathBuf::new(),
        });
        SkillRegistry::from_map(skills, PathBuf::from("skills"))
    }

    #[test]
    fn test_search_by_text_tag_match() {
        let reg = make_registry();
        let results = search_by_text(&reg, "infrastructure", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "cloudflare-dns");
    }

    #[test]
    fn test_search_by_text_name_match() {
        let reg = make_registry();
        let results = search_by_text(&reg, "github", 10);
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "github");
    }

    #[test]
    fn test_search_by_tags() {
        let reg = make_registry();
        let results = search_by_tags(&reg, &["development".to_string()], 10);
        assert_eq!(results.len(), 2); // github + debugging both have "development"
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }
}
