//! `map` describes the project's own code; type declarations the indexer
//! pulls from installed packages stay out unless `--module` asks for them.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn run(root: &Path, cache: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_ast-index"))
        .current_dir(root)
        .env("AST_INDEX_CACHE_DIR", cache)
        .env("AST_INDEX_DISABLE_GC", "1")
        .env("NO_COLOR", "1")
        .env_remove("AST_INDEX_DB_PATH")
        .env_remove("KOTLIN_INDEX_DB_PATH")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn project_with_installed_package() -> (TempDir, TempDir) {
    let project = TempDir::new().unwrap();
    let cache = TempDir::new().unwrap();
    let root = project.path();
    fs::write(root.join("package.json"), "{}\n").unwrap();
    fs::create_dir_all(root.join("src/billing")).unwrap();
    fs::write(
        root.join("src/billing/invoice.ts"),
        "export class Invoice {}\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("node_modules/@types/money")).unwrap();
    fs::write(
        root.join("node_modules/@types/money/index.d.ts"),
        "export declare class Money {}\nexport interface Currency {}\n",
    )
    .unwrap();
    run(root, cache.path(), &["rebuild"]);
    (project, cache)
}

#[test]
fn map_leaves_installed_packages_out_of_the_summary() {
    let (project, cache) = project_with_installed_package();

    let text = run(project.path(), cache.path(), &["map"]);
    assert!(text.contains("src/billing/"), "{text}");
    assert!(!text.contains("node_modules"), "{text}");
    assert!(
        text.contains("1 files (+1 dependency type declarations)"),
        "{text}"
    );

    let json: serde_json::Value = serde_json::from_str(&run(
        project.path(),
        cache.path(),
        &["--format", "json", "map"],
    ))
    .unwrap();
    assert_eq!(json["file_count"], 1);
    assert_eq!(json["dependency_file_count"], 1);
    let groups = json["groups"].as_array().unwrap();
    assert!(
        groups
            .iter()
            .all(|g| !g["path"].as_str().unwrap().contains("node_modules")),
        "{json}"
    );
}

#[test]
fn map_module_outside_packages_skips_them_too() {
    let (project, cache) = project_with_installed_package();

    let text = run(project.path(), cache.path(), &["map", "-m", ""]);
    assert!(text.contains("Invoice"), "{text}");
    assert!(!text.contains("Money"), "{text}");
}

#[test]
fn map_module_inside_a_package_shows_it() {
    let (project, cache) = project_with_installed_package();

    let text = run(
        project.path(),
        cache.path(),
        &["map", "-m", "node_modules/@types/money"],
    );
    assert!(text.contains("Money"), "{text}");
    assert!(!text.contains("Invoice"), "{text}");
}
