# Change Log

All user visible changes to this project will be documented in this file.
This project adheres to [Semantic Versioning](http://semver.org/), as described
for Rust libraries in [RFC #1105](https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md)

## Unreleased

### Added

N/A

### Changed

N/A

### Removed

N/A

### Fixed

N/A

## [1.0.0] - 2019-02-15

### Fixed

- Make macros work with trailing comma

## [0.2.1] - 2018-11-15

### Fixed

- Fix wrong error message when a JSON atom was missing from actual.

## [0.2.0] - 2018-11-16

### Added

- Add `assert_json_include`. It does partial matching the same way the old `assert_json_eq` did.

### Changed

- Change `assert_json_eq` do exact matching. If the two values are not exactly the same, it'll panic.

## 0.1.0 - 2018-10-17

Initial release.

[1.0.0]: https://github.com/davidpdrsn/assert-json-diff/compare/v0.2.1...v1.0.0
[0.2.1]: https://github.com/davidpdrsn/assert-json-diff/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/davidpdrsn/assert-json-diff/compare/v0.1.0...v0.2.0
