#[test]
fn release_workflow_publishes_archives_and_checksums() {
    let workflow = std::fs::read_to_string(".github/workflows/release.yml").unwrap();

    assert!(workflow.contains("contents: write"));
    assert!(workflow.contains("v*.*.*"));
    assert!(workflow.contains("x86_64-unknown-linux-gnu"));
    assert!(workflow.contains("x86_64-apple-darwin"));
    assert!(workflow.contains("aarch64-apple-darwin"));
    assert!(workflow.contains("x86_64-pc-windows-msvc"));
    assert!(workflow.contains(".tar.gz"));
    assert!(workflow.contains(".zip"));
    assert!(workflow.contains("SHA256SUMS.txt"));
    assert!(workflow.contains("gh release upload"));
}

#[test]
fn release_docs_describe_install_and_verification() {
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    let readme = std::fs::read_to_string("README.md").unwrap();
    let checklist = std::fs::read_to_string("RELEASE.md").unwrap();

    assert!(manifest.contains("version = \"0.2.0\""));
    assert!(readme.contains("GitHub Releases"));
    assert!(readme.contains("SHA256SUMS.txt"));
    assert!(readme.contains("codex-receipt-x86_64-pc-windows-msvc.zip"));
    assert!(checklist.contains("cargo fmt --check"));
    assert!(checklist.contains("cargo clippy --all-targets -- -D warnings"));
    assert!(checklist.contains("cargo test"));
    assert!(checklist.contains("git tag v0.2.0"));
}
