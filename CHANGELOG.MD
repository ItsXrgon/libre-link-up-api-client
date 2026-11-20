# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial Rust implementation of LibreLinkUp API client
- Automatic authentication and token management
- Regional endpoint handling (US, EU, JP, DE, FR, AP, AU, AE)
- Current and historical glucose data reading
- Raw API response access
- Averaged glucose readings over time
- Comprehensive error handling
- Full async/await support with Tokio
- Type-safe API with proper Rust types

## [0.4.1] - 2026-11-20

### Fixed
- Login now returns `LibreLinkUpError::AdditionalActionRequired` when the account must accept
  updated terms of use (`"tou"`) or privacy policy (`"pp"`), instead of
  `InvalidResponse("Failed to parse JSON: data did not match any variant ...")`. The login
  body is now classified by `status` and a few known fields before any typed
  deserialization, and the step name comes from `step.componentName`, falling back to
  `step.type`, then `"unknown"`.
- Unrecognized login responses now produce `InvalidResponse` naming the login status and the
  keys under `data`. Values from the body (tokens, emails, ...) are never included.

### Changed
- `StepData`, `Step`, `StepProps` and `StepUser` accept step responses with missing fields:
  everything except `step` defaults when absent. Field types are unchanged.
  `AuthTicket`, `Step`, `StepProps` and `StepUser` now implement `Default`.

## [0.4.0] - 2026-11-20

### Fixed
- Glucose values from `read()` are now always in mg/dL, as documented. They were taken from
  the API's `Value` field, which is in the account's display unit, so mmol/L accounts got
  ~6.2 instead of ~112. They now come from `ValueInMgPerDl`.
- Readings with an unparseable `FactoryTimestamp` are no longer stamped with `Utc::now()`.
  `read()` returns `LibreLinkUpError::InvalidResponse` if the latest reading's timestamp
  cannot be parsed, and leaves out history points whose timestamp cannot be parsed.
- A missing, negative or out-of-range `TrendArrow` now maps to `TrendType::NotComputable`
  instead of `TrendType::Flat`.
- Averaged polling no longer counts `NotComputable` readings in the trend mean (they pulled
  it toward "falling"). If no reading has a computable trend, the result is `NotComputable`
  (was `Flat`).
- `ClientConfig::api_version` with characters that are invalid in an HTTP header now returns
  an error instead of panicking.
- Documentation fixes: the averaged-polling callback arguments were described in the wrong
  order, `ConnectionIdentifier::ByName` matches the full name only (not first or last name),
  and `TrendType` arrows are single arrows (↓/↑), not double.

### Changed (breaking)
- `LibreCgmData.value` is renamed to `value_mg_dl` (serialized as `valueMgDl`).
- `read_averaged(amount, callback, interval_ms)` is renamed to
  `poll_averaged(sample_count, interval: Duration, on_average)`. It returns
  `LibreLinkUpError::InvalidConfig` if `sample_count` or `interval` is zero; before, these
  produced a NaN average or a panic.
- `LibreLinkUpClient::simple` takes `region: Option<&str>` instead of `Option<String>`.
- `LibreLinkUpError`:
  - `AuthFailed` is replaced by `InvalidConfig`. It was only ever returned for an empty
    username or password, never for a failed login.
  - `RegionNotFound` and `SerializationError` are removed; neither was ever returned.
- The `utils` module is now private. Convert raw readings with
  `LibreCgmData::try_from(&GlucoseItem)` or `LibreCgmData::try_from(&GlucoseMeasurement)`.

### Added
- `TryFrom<&GlucoseItem>` and `TryFrom<&GlucoseMeasurement>` for `LibreCgmData`.
- `GlucoseMeasurement` and `DEFAULT_API_VERSION` are re-exported at the crate root.
- Documentation for every error variant and every `ClientConfig` field.

## [0.1.0] - 2026-02-07

### Added
- Initial release


# 0.1.0 (2022-05-27)

### Features
