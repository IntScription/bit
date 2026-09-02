//! Validates /content against the exact schema and graph rules the app
//! loads at runtime. See /ARCHITECTURE.md §9.
//!
//! Usage: `cargo run -p content-lint -- [path-to-content-dir]`
//! Defaults to `./content` relative to the current directory.

use bit_build::catalog;
use bit_content::{load_all, FilesystemSource};

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| "content".to_string());
    let source = FilesystemSource::new(&root);

    let loaded = match load_all(&source) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("content-lint failed against '{root}': {e:?}");
            std::process::exit(1);
        }
    };

    // Parsing a BuildDefinition only proves its RON is well-formed, not
    // that its params actually drive its simulation (e.g. a missing or
    // misspelled "bit_width"). Instantiating every build here catches that
    // class of error at lint time instead of at first playtest.
    let mut failed = false;
    for build in &loaded.builds {
        if let Err(e) = catalog::instantiate(build) {
            eprintln!("content-lint: build '{}' failed to instantiate: {e:?}", build.id);
            failed = true;
        }
    }
    if failed {
        std::process::exit(1);
    }

    println!(
        "content OK: loaded from '{root}' ({} domains, {} concepts, {} builds, all instantiable)",
        loaded.graph.domain_count(),
        loaded.graph.concept_count(),
        loaded.builds.len(),
    );
}
