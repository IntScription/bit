//! Loads and validates on-disk content (RON metadata + Markdown prose)
//! into the typed `bit-knowledge` graph. See /ARCHITECTURE.md §3 and §9.
//!
//! Nothing outside this crate should deserialize a content file directly
//! — `content-lint` reuses this same code path so the validator can never
//! drift from what the app actually loads at runtime.

use std::fs;
use std::path::Path;

use bit_build::BuildDefinition;
use bit_knowledge::{Concept, Domain, GraphError, KnowledgeGraph};

#[derive(Debug, PartialEq, Eq)]
pub enum ContentError {
    Io(String),
    Parse { file: String, message: String },
    Graph(GraphError),
    /// A build's `concept_ids` names a concept the knowledge graph doesn't
    /// have — distinct from `GraphError::DanglingReference` (which is
    /// concept-to-concept) so lint output points at the right content kind.
    DanglingBuildReference { build: String, concept: String },
}

impl From<GraphError> for ContentError {
    fn from(e: GraphError) -> Self {
        ContentError::Graph(e)
    }
}

/// Where content files come from. `FilesystemSource` is used in dev and by
/// `content-lint`; the iOS app will add a `BundleSource` reading app
/// resources without changing any loading logic below.
pub trait ContentSource {
    /// Returns the raw text of every file with `extension` found
    /// (recursively) under `subdir`.
    fn read_all(&self, subdir: &str, extension: &str) -> Result<Vec<(String, String)>, ContentError>;
}

pub struct FilesystemSource {
    root: std::path::PathBuf,
}

impl FilesystemSource {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ContentSource for FilesystemSource {
    fn read_all(&self, subdir: &str, extension: &str) -> Result<Vec<(String, String)>, ContentError> {
        let dir = self.root.join(subdir);
        let mut out = Vec::new();
        if dir.exists() {
            walk(&dir, extension, &mut out)?;
        }
        Ok(out)
    }
}

fn walk(dir: &Path, extension: &str, out: &mut Vec<(String, String)>) -> Result<(), ContentError> {
    let entries = fs::read_dir(dir).map_err(|e| ContentError::Io(e.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|e| ContentError::Io(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            walk(&path, extension, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some(extension) {
            let text = fs::read_to_string(&path).map_err(|e| ContentError::Io(e.to_string()))?;
            out.push((path.display().to_string(), text));
        }
    }
    Ok(())
}

/// In-memory source, useful for tests and for content embedded at compile
/// time (e.g. via `include_str!`) instead of read from disk.
#[derive(Default)]
pub struct MemorySource {
    files: Vec<(String, String, String)>, // (subdir, extension, text)
}

impl MemorySource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(mut self, subdir: &str, extension: &str, text: impl Into<String>) -> Self {
        self.files.push((subdir.to_string(), extension.to_string(), text.into()));
        self
    }
}

impl ContentSource for MemorySource {
    fn read_all(&self, subdir: &str, extension: &str) -> Result<Vec<(String, String)>, ContentError> {
        Ok(self
            .files
            .iter()
            .filter(|(s, e, _)| s == subdir && e == extension)
            .enumerate()
            .map(|(i, (_, _, text))| (format!("{subdir}/mem-{i}.{extension}"), text.clone()))
            .collect())
    }
}

pub fn load_knowledge_graph(source: &dyn ContentSource) -> Result<KnowledgeGraph, ContentError> {
    let domains = source
        .read_all("domains", "ron")?
        .into_iter()
        .map(|(file, text)| {
            ron::from_str::<Domain>(&text).map_err(|e| ContentError::Parse { file, message: e.to_string() })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let concepts = source
        .read_all("concepts", "ron")?
        .into_iter()
        .map(|(file, text)| {
            ron::from_str::<Concept>(&text).map_err(|e| ContentError::Parse { file, message: e.to_string() })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(KnowledgeGraph::build(domains, concepts)?)
}

pub fn load_builds(source: &dyn ContentSource) -> Result<Vec<BuildDefinition>, ContentError> {
    source
        .read_all("builds", "ron")?
        .into_iter()
        .map(|(file, text)| {
            ron::from_str::<BuildDefinition>(&text).map_err(|e| ContentError::Parse { file, message: e.to_string() })
        })
        .collect()
}

/// Everything `bit-core` needs at startup, loaded and cross-validated
/// together: builds are checked against the graph they were loaded
/// alongside, so a build referencing a nonexistent concept fails to load
/// the same way a concept referencing a nonexistent prerequisite does.
#[derive(Debug)]
pub struct LoadedContent {
    pub graph: KnowledgeGraph,
    pub builds: Vec<BuildDefinition>,
}

pub fn load_all(source: &dyn ContentSource) -> Result<LoadedContent, ContentError> {
    let graph = load_knowledge_graph(source)?;
    let builds = load_builds(source)?;

    for build in &builds {
        for concept_id in &build.concept_ids {
            if graph.concept(concept_id).is_none() {
                return Err(ContentError::DanglingBuildReference {
                    build: build.id.clone(),
                    concept: concept_id.clone(),
                });
            }
        }
    }

    Ok(LoadedContent { graph, builds })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_a_minimal_graph_from_memory() {
        let source = MemorySource::new()
            .add(
                "domains",
                "ron",
                r#"Domain(id: "foundations", name: "Foundations", parent: None, order: 0)"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "bits",
                    title: "Bits",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: [],
                    related: [],
                    learning_objectives: ["Understand a bit as a single binary digit"],
                    explanation: "bits.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: ["Digital electronics"],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: None),
                )"#,
            );

        let graph = load_knowledge_graph(&source).expect("graph should load");
        assert!(graph.concept("bits").is_some());
        assert_eq!(graph.concepts_in("foundations").len(), 1);
    }

    fn source_with_build(build_ron: &str) -> MemorySource {
        MemorySource::new()
            .add(
                "domains",
                "ron",
                r#"Domain(id: "foundations", name: "Foundations", parent: None, order: 0)"#,
            )
            .add(
                "concepts",
                "ron",
                r#"Concept(
                    id: "bits",
                    title: "Bits",
                    domain: "foundations",
                    difficulty: Easy,
                    applicability: [Core],
                    prerequisites: [],
                    related: [],
                    learning_objectives: [],
                    explanation: "bits.md",
                    interactive_demo: None,
                    practice_types: [],
                    build_refs: [],
                    lab_capabilities: [],
                    real_world_uses: [],
                    historical_relevance: None,
                    mastery: (min_practice_correct: 3, min_accuracy: 0.8, decay_days: None),
                )"#,
            )
            .add("builds", "ron", build_ron)
    }

    #[test]
    fn load_all_succeeds_when_build_references_an_existing_concept() {
        let source = source_with_build(
            r#"BuildDefinition(
                id: "binary_counter",
                name: "Binary Counter",
                tier: Beginner,
                description: "Count in binary",
                concept_ids: ["bits"],
                params: {"bit_width": "8", "target_value": "10"},
                freeform: false,
            )"#,
        );

        let loaded = load_all(&source).expect("load_all should succeed");
        assert_eq!(loaded.builds.len(), 1);
        assert!(loaded.graph.concept("bits").is_some());
    }

    #[test]
    fn load_all_rejects_a_build_referencing_an_unknown_concept() {
        let source = source_with_build(
            r#"BuildDefinition(
                id: "binary_counter",
                name: "Binary Counter",
                tier: Beginner,
                description: "Count in binary",
                concept_ids: ["does_not_exist"],
                params: {},
                freeform: false,
            )"#,
        );

        let err = load_all(&source).expect_err("load_all should reject a dangling build reference");
        assert_eq!(
            err,
            ContentError::DanglingBuildReference {
                build: "binary_counter".into(),
                concept: "does_not_exist".into(),
            }
        );
    }
}
