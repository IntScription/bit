//! Core knowledge-graph types and the in-memory graph engine that Explore
//! navigates. See /ARCHITECTURE.md §2 and §6.
//!
//! `bit-knowledge` owns the *types*; `bit-content` is responsible for
//! parsing RON/Markdown files on disk into these types and handing a
//! `Vec<Domain>` + `Vec<Concept>` to `KnowledgeGraph::build`.

use std::collections::HashMap;

pub type ConceptId = String;
pub type DomainId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Difficulty {
    Easy,
    Core,
    Practical,
    Advanced,
    Specialized,
    Extreme,
    Theoretical,
}

impl Difficulty {
    /// Ordinal matching spec §5's stated progression path (EASY → CORE →
    /// PRACTICAL → ADVANCED → SPECIALIZED → EXTREME → THEORETICAL/
    /// OBSCURE/EXPERIMENTAL). Used to pick a sensible default ordering
    /// among several "ready to learn" candidates — not a claim that
    /// harder always means more useful (§5 explicitly says the opposite),
    /// just a reasonable default when nothing else disambiguates.
    pub fn rank(self) -> u8 {
        match self {
            Difficulty::Easy => 0,
            Difficulty::Core => 1,
            Difficulty::Practical => 2,
            Difficulty::Advanced => 3,
            Difficulty::Specialized => 4,
            Difficulty::Extreme => 5,
            Difficulty::Theoretical => 6,
        }
    }
}

/// Applicability classification — a concept can carry more than one.
/// See /ARCHITECTURE.md and spec §5: advanced does not imply useful,
/// and rare/theoretical concepts are first-class, not lesser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Applicability {
    Core,
    Practical,
    Specialized,
    Rare,
    Theoretical,
    Historical,
    Emerging,
    Experimental,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MasteryRule {
    pub min_practice_correct: u32,
    pub min_accuracy: f32,
    pub decay_days: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InteractiveDemo {
    /// Key into a client-side registry of native interactive components.
    /// bit-knowledge never interprets this string — it only carries it.
    pub id: String,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Domain {
    pub id: DomainId,
    pub name: String,
    pub parent: Option<DomainId>,
    pub order: i32,
    /// One line describing what this domain covers — shown as a
    /// subtitle under the folder name in Explore. `#[serde(default)]`
    /// so it's optional in `.ron` authoring, not a required field.
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Concept {
    pub id: ConceptId,
    pub title: String,
    pub domain: DomainId,
    pub difficulty: Difficulty,
    pub applicability: Vec<Applicability>,
    pub prerequisites: Vec<ConceptId>,
    pub related: Vec<ConceptId>,
    pub learning_objectives: Vec<String>,
    pub explanation: String,
    pub interactive_demo: Option<InteractiveDemo>,
    pub practice_types: Vec<String>,
    pub build_refs: Vec<String>,
    pub lab_capabilities: Vec<String>,
    pub real_world_uses: Vec<String>,
    pub historical_relevance: Option<String>,
    pub mastery: MasteryRule,
}

#[derive(Debug, Clone)]
pub struct SearchHit {
    pub concept_id: ConceptId,
    pub score: u32,
}

/// Error produced while assembling the graph from loaded content.
/// `bit-content`'s lint tool surfaces these to content authors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    DuplicateConcept(ConceptId),
    DuplicateDomain(DomainId),
    UnknownDomain { concept: ConceptId, domain: DomainId },
    UnknownParentDomain { domain: DomainId, parent: DomainId },
    DanglingReference { concept: ConceptId, references: ConceptId },
}

/// The assembled, queryable knowledge graph. Built once at startup from
/// content loaded by `bit-content`; read-only thereafter.
#[derive(Debug)]
pub struct KnowledgeGraph {
    domains: HashMap<DomainId, Domain>,
    domain_children: HashMap<DomainId, Vec<DomainId>>,
    concepts: HashMap<ConceptId, Concept>,
    concepts_by_domain: HashMap<DomainId, Vec<ConceptId>>,
    used_by: HashMap<ConceptId, Vec<ConceptId>>,
    search_index: HashMap<String, Vec<ConceptId>>,
}

impl KnowledgeGraph {
    pub fn build(domains: Vec<Domain>, concepts: Vec<Concept>) -> Result<Self, GraphError> {
        let mut domain_map = HashMap::new();
        for d in domains {
            if domain_map.insert(d.id.clone(), d).is_some() {
                let id = domain_map.keys().next().cloned().unwrap_or_default();
                return Err(GraphError::DuplicateDomain(id));
            }
        }
        for d in domain_map.values() {
            if let Some(parent) = &d.parent {
                if !domain_map.contains_key(parent) {
                    return Err(GraphError::UnknownParentDomain {
                        domain: d.id.clone(),
                        parent: parent.clone(),
                    });
                }
            }
        }

        let mut domain_children: HashMap<DomainId, Vec<DomainId>> = HashMap::new();
        for d in domain_map.values() {
            if let Some(parent) = &d.parent {
                domain_children.entry(parent.clone()).or_default().push(d.id.clone());
            }
        }

        let mut concept_map: HashMap<ConceptId, Concept> = HashMap::new();
        for c in concepts {
            if concept_map.contains_key(&c.id) {
                return Err(GraphError::DuplicateConcept(c.id));
            }
            if !domain_map.contains_key(&c.domain) {
                return Err(GraphError::UnknownDomain {
                    concept: c.id,
                    domain: c.domain,
                });
            }
            concept_map.insert(c.id.clone(), c);
        }

        // Validate prerequisite/related references resolve, and build the
        // reverse "used_by" index while we're at it.
        let mut used_by: HashMap<ConceptId, Vec<ConceptId>> = HashMap::new();
        for c in concept_map.values() {
            for reference in c.prerequisites.iter().chain(c.related.iter()) {
                if !concept_map.contains_key(reference) {
                    return Err(GraphError::DanglingReference {
                        concept: c.id.clone(),
                        references: reference.clone(),
                    });
                }
                used_by.entry(reference.clone()).or_default().push(c.id.clone());
            }
        }

        let mut concepts_by_domain: HashMap<DomainId, Vec<ConceptId>> = HashMap::new();
        for c in concept_map.values() {
            concepts_by_domain.entry(c.domain.clone()).or_default().push(c.id.clone());
        }

        let mut search_index: HashMap<String, Vec<ConceptId>> = HashMap::new();
        for c in concept_map.values() {
            for token in tokenize(&c.title) {
                search_index.entry(token).or_default().push(c.id.clone());
            }
        }

        Ok(Self {
            domains: domain_map,
            domain_children,
            concepts: concept_map,
            concepts_by_domain,
            used_by,
            search_index,
        })
    }

    pub fn domain(&self, id: &str) -> Option<&Domain> {
        self.domains.get(id)
    }

    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Top-level domains (no parent) — where Explore's tree navigation
    /// starts.
    pub fn root_domains(&self) -> Vec<&Domain> {
        let mut roots: Vec<&Domain> = self.domains.values().filter(|d| d.parent.is_none()).collect();
        roots.sort_by_key(|d| (d.order, d.id.clone()));
        roots
    }

    pub fn children(&self, domain: &str) -> Vec<&Domain> {
        let mut children: Vec<&Domain> = self
            .domain_children
            .get(domain)
            .map(|ids| ids.iter().filter_map(|id| self.domains.get(id)).collect())
            .unwrap_or_default();
        children.sort_by_key(|d| (d.order, d.id.clone()));
        children
    }

    pub fn concepts_in(&self, domain: &str) -> Vec<&Concept> {
        let mut concepts: Vec<&Concept> = self
            .concepts_by_domain
            .get(domain)
            .map(|ids| ids.iter().filter_map(|id| self.concepts.get(id)).collect())
            .unwrap_or_default();
        concepts.sort_by_key(|c| c.title.clone());
        concepts
    }

    pub fn concept(&self, id: &str) -> Option<&Concept> {
        self.concepts.get(id)
    }

    /// Every concept in the graph, regardless of domain. Profile uses this
    /// to summarize discovery/mastery across the whole graph without
    /// having to walk the domain tree itself.
    pub fn all_concepts(&self) -> Vec<&Concept> {
        self.concepts.values().collect()
    }

    pub fn prerequisites(&self, id: &str) -> Vec<&Concept> {
        self.concepts
            .get(id)
            .map(|c| c.prerequisites.iter().filter_map(|p| self.concepts.get(p)).collect())
            .unwrap_or_default()
    }

    pub fn related(&self, id: &str) -> Vec<&Concept> {
        self.concepts
            .get(id)
            .map(|c| c.related.iter().filter_map(|p| self.concepts.get(p)).collect())
            .unwrap_or_default()
    }

    pub fn used_by(&self, id: &str) -> Vec<&Concept> {
        self.used_by
            .get(id)
            .map(|ids| ids.iter().filter_map(|p| self.concepts.get(p)).collect())
            .unwrap_or_default()
    }

    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let mut scores: HashMap<ConceptId, u32> = HashMap::new();
        for token in tokenize(query) {
            if let Some(ids) = self.search_index.get(&token) {
                for id in ids {
                    *scores.entry(id.clone()).or_insert(0) += 1;
                }
            }
        }
        let mut hits: Vec<SearchHit> = scores
            .into_iter()
            .map(|(concept_id, score)| SearchHit { concept_id, score })
            .collect();
        hits.sort_by(|a, b| b.score.cmp(&a.score).then(a.concept_id.cmp(&b.concept_id)));
        hits
    }
}

fn tokenize(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mastery() -> MasteryRule {
        MasteryRule { min_practice_correct: 3, min_accuracy: 0.8, decay_days: None }
    }

    fn concept(id: &str, domain: &str, prerequisites: Vec<&str>) -> Concept {
        Concept {
            id: id.to_string(),
            title: id.to_string(),
            domain: domain.to_string(),
            difficulty: Difficulty::Core,
            applicability: vec![Applicability::Core],
            prerequisites: prerequisites.into_iter().map(String::from).collect(),
            related: vec![],
            learning_objectives: vec![],
            explanation: format!("{id}.md"),
            interactive_demo: None,
            practice_types: vec![],
            build_refs: vec![],
            lab_capabilities: vec![],
            real_world_uses: vec![],
            historical_relevance: None,
            mastery: mastery(),
        }
    }

    fn sample_graph() -> KnowledgeGraph {
        let domains = vec![
            Domain { id: "foundations".into(), name: "Foundations".into(), parent: None, order: 0, description: String::new() },
            Domain {
                id: "foundations.bits_binary".into(),
                name: "Bits & Binary".into(),
                parent: Some("foundations".into()),
                order: 0,
                description: String::new(),
            },
        ];
        let concepts = vec![
            concept("bits", "foundations.bits_binary", vec![]),
            concept("binary", "foundations.bits_binary", vec!["bits"]),
        ];
        KnowledgeGraph::build(domains, concepts).unwrap()
    }

    #[test]
    fn root_domains_returns_only_parentless_domains() {
        let graph = sample_graph();
        let roots = graph.root_domains();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].id, "foundations");
    }

    #[test]
    fn builds_domain_tree() {
        let graph = sample_graph();
        let children = graph.children("foundations");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, "foundations.bits_binary");
    }

    #[test]
    fn resolves_prerequisites_and_reverse_used_by() {
        let graph = sample_graph();
        let prereqs = graph.prerequisites("binary");
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].id, "bits");

        let used_by = graph.used_by("bits");
        assert_eq!(used_by.len(), 1);
        assert_eq!(used_by[0].id, "binary");
    }

    #[test]
    fn all_concepts_returns_every_concept_regardless_of_domain() {
        let graph = sample_graph();
        let mut ids: Vec<_> = graph.all_concepts().iter().map(|c| c.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, vec!["binary".to_string(), "bits".to_string()]);
    }

    #[test]
    fn search_finds_by_title_token() {
        let graph = sample_graph();
        let hits = graph.search("binary");
        assert_eq!(hits[0].concept_id, "binary");
    }

    #[test]
    fn rejects_dangling_prerequisite() {
        let domains = vec![Domain {
            id: "foundations".into(),
            name: "Foundations".into(),
            parent: None,
            order: 0,
            description: String::new(),
        }];
        let concepts = vec![concept("binary", "foundations", vec!["bits"])];
        let err = KnowledgeGraph::build(domains, concepts).unwrap_err();
        assert_eq!(
            err,
            GraphError::DanglingReference { concept: "binary".into(), references: "bits".into() }
        );
    }
}
