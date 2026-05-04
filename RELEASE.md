# codex-receipt Release Checklist

Use this checklist before creating a v2 release tag.

## Local Verification

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] Confirm `Cargo.toml` has the intended release version.
- [ ] Confirm `README.md` install commands match the artifact names in `.github/workflows/release.yml`.

## Tag And Publish

- [ ] Create the release tag: `git tag v0.2.0`.
- [ ] Push the release tag: `git push origin v0.2.0`.
- [ ] Wait for the `Release` workflow to finish.
- [ ] Confirm the GitHub Release contains all four platform archives.
- [ ] Confirm `SHA256SUMS.txt` is attached to the GitHub Release.

## Smoke Test

- [ ] Download one archive from GitHub Releases.
- [ ] Verify the archive checksum with `SHA256SUMS.txt`.
- [ ] Extract the binary.
- [ ] Run `codex-receipt --session tests/fixtures/codex-session.jsonl`.
- [ ] Confirm the command prints a Chinese receipt.
