# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/NickLarsenNZ/cool-diff/releases/tag/v0.1.0) - 2026-04-20

### Added

- Add support for terminal colors
- Enable truncation of long arrays and objects
- Render subtree for the missing case
- Implement type mismatch rendering
- Add truncation markers for fields/items omitted (we treat that as noise)
- Add trait for diff renderers
- Implement Ambiguous match handling
- Implement array contains based comparison
- Implement array key based comparison
- Implement array index based comparison
- Implement object comparison
- Compare scalar values, and return DiffResult from diff_values
- Add the main diff entry point, and scaffolding for the diff algorithm
- Add DiffConfig
- Add DiffTree and dependent types

### Fixed

- Use correct terms for the omitted siblings
- Rendering of array index comments
- Rendering of array segments
- Only add trailing `:` when the path segment is a key

### Other

- Update feature list
- Keep the release light
- Update readme to link to CHANGELOG.md and RELEASE,md
- Add release workflow and instructions
- Set up Dependabot for automated dependency updates
- Bump actions
- Add MSRV check
- Show how to run examples with colors
- Use singular form for omission units when there is a count of one
- Move tests to end of file
- Use rustup instead of dtolnay/rust-toolchain
- Fix cargo deny rules for v2
- Replace rustsec/audit-check with cargo audit
- Add scheduled security audit workflow
- Add cargo deny config
- Drop permissions and add them as necessary with justifications
- Pin actions to hashes
- Add workflow
- Add module and item docs
- Use builder methods and remove pub access to fields
- disallow unwrap
- Bump deps
- Move imports to top of examples
- Add strict example
- Add error handling
- Add readme
- Add licenses
- Add a logo
- Add example for a custom renderer
- Add runnable examples
- Rename indicator::{UNCHANGED -> CONTEXT}
- Derive default on AmbiguousMatchStrategy
- Add kubernetes fixture tests
- Check for null compared with empty array or object
- Add additional tests
- Deduplicate render code
- Deduplicate type mismatch comment
- Implement yaml diff renderer
- Get rid of warning
- Point out the differences in the expected documents
- Add fixtures with actual and expected documents per file
- Remove dead code
- Make use of PathSegment::Unmatched for key based array comparisons
- Format code
- Add constructors to make code neater
- Add unit tests
- Improve the previous commit do allow per-path ambiguity handling too
- Allow for more granular array matching modes
- *(cargo)* Update toolchain to 1.95
- cargo init
