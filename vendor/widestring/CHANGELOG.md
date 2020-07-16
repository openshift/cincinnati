# Changelog

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- New `U32String`, `U32Str`, `U32CString`, and `U32CStr` types for dealing with UTF-32 FFI. These
  new types are roughly equivalent to the existing UTF-16 types.
- `WideChar` is a type alias to `u16` on Windows but `u32` on non-Windows platforms.
- The generic types `UString`, `UStr`, `UCString` and `UCStr` are used to implement the string
  types.

### Changed
- **Breaking Change** Existing wide string types have been renamed to `U16String`, `U16Str`,
  `U16CString`, and `U16CStr` (previously `WideString`, `WideStr`, etc.). Some function have
  also been renamed to reflect this change (`wide_str` to `u16_str`, etc.).
- **Breaking Change** `WideString`, `WideStr`, `WideCString`, and `WideCStr` are now type aliases
  that vary between platforms. On Windows, these are aliases to the `U16` types and are equivalent
  to the previous version, but on non-Windows platforms these alias the new `U32` types instead.
  See crate documentation for more details.

## [0.3.0] - 2018-03-17 <a name="0.3.0"></a>
### Added
- Additional unchecked functions on `WideCString`.
- All types now implement `Default`.
- `WideString::shrink_to_fit`
- `WideString::into_boxed_wide_str` and `Box<WideStr>::into_wide_string`.
- `WideCString::into_boxed_wide_c_str` and `Box<WideCStr>::into_wide_c_string`.
- `From` and `Default` implementations for boxed `WideStr` and boxed `WideCStr`.

### Changed
- Renamed `WideCString::from_vec` to replace `WideCString::new`. To create empty string, use
  `WideCString::default()` now.
- `WideCString` now implements `Drop`, which sets the string to an empty string to prevent invalid
  unsafe code from working correctly when it should otherwise break. Also see `Drop` implementation
  of `CString`.
- Writing changelog manually.
- Upgraded winapi dev dependency.
- Now requires at least Rust 1.17+ to compile (previously, was Rust 1.8).

## [0.2.2] - 2016-09-09 <a name="0.2.2"></a>
### Fixed
- Make `WideCString::into_raw` correctly forget the original self.

## [0.2.1] - 2016-08-12 <a name="0.2.1"></a>
### Added
- `into_raw`/`from_raw` on `WideCString`. Closes [#2].

## [0.2.0] - 2016-05-31 <a name="0.2.0"></a>
### Added
- `Default` trait to wide strings.
- Traits for conversion of strings to `Cow`.
### Changed
- Methods & traits to bring to parity with Rust 1.9 string APIs.

## 0.1.0 - 2016-02-06 <a name="0.1.0"></a>
### Added
- Initial release.

[#2]: https://github.com/starkat99/widestring-rs/issues/2

[Unreleased]: https://github.com/starkat99/widestring-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/starkat99/widestring-rs/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/starkat99/widestring-rs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/starkat99/widestring-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/starkat99/widestring-rs/compare/v0.1.0...v0.2.0
