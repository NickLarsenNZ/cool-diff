# Release Process

This project uses [release-plz](https://release-plz.dev/) to automate releases.

## How it works

1. Merge PRs to `main` using [conventional commits](https://www.conventionalcommits.org/) in the merge commit or PR commits.
2. release-plz automatically creates or updates a release PR with:
   - A version bump in `Cargo.toml` (based on commit types)
   - An updated `CHANGELOG.md` with entries from new commits
3. The release PR stays open and accumulates changes as more commits land on `main`.
4. When you're ready to release, review and merge the release PR.
5. On merge, release-plz automatically:
   - Creates a git tag (e.g. `v0.2.0`)
   - Creates a GitHub release with the changelog
   - Publishes the crate to crates.io

## Conventional commits

The version bump is determined by commit types:

| Commit prefix | Version bump | Example |
|---|---|---|
| `fix:` | Patch (0.1.0 -> 0.1.1) | `fix: Handle empty arrays in contains mode` |
| `feat:` | Minor (0.1.0 -> 0.2.0) | `feat: Add JSON renderer` |
| `feat!:` or `BREAKING CHANGE:` | Major (0.1.0 -> 1.0.0) | `feat!: Make diff() return Result` |
| Other (e.g. `chore:`, `docs:`, `test:`, `refactor:`) | Patch | `docs: Update README examples` |

## Prerequisites

Before the first publish, ensure:

- The GitHub repo is public (crates.io requires this)
- A `CARGO_REGISTRY_TOKEN` secret is set in the repo settings (generate at https://crates.io/settings/tokens with `publish-new` and `publish-update` scopes)

## MSRV bumps

The MSRV should only be bumped when necessary:

- A dependency update (especially a security fix) requires a newer Rust version
- A new feature needs a language or stdlib addition not available in the current MSRV

Avoid bumping MSRV just for convenience or minor ergonomic improvements.

When bumping:

- Update `rust-version` in `Cargo.toml`
- Update the MSRV in `.github/workflows/ci.yml` (the `msrv` job)
- Use a `feat(msrv):` commit (it's a minor version change per Rust ecosystem convention)
- Mention the reason in the commit message, e.g. `feat(msrv): Bump to 1.90 (required by snafu 0.10)`

## Manual release (if needed)

If automation isn't available, you can release manually:

```sh
# Bump version in Cargo.toml
# Update CHANGELOG.md
git tag v0.x.y
git push --tags
cargo publish
```
