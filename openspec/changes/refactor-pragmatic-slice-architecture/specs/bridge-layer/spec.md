## MODIFIED Requirements

### Requirement: Bridge Runtime Execution Contract

The bridge layer SHALL enforce a unified runtime strategy for sync and async FFI entry points.

#### Scenario: Sync FFI uses runtime guard blocking path

- **WHEN** a synchronous FFI API is invoked
- **THEN** execution goes through runtime guard blocking interface
- **AND** nested `block_on` is not performed if a Tokio runtime context already exists

#### Scenario: Async FFI awaits directly

- **WHEN** an asynchronous FFI API is invoked
- **THEN** the API directly `await`s async operations
- **AND** no extra blocking wrapper is introduced

### Requirement: Bridge Error Mapping Contract

The bridge layer SHALL map errors by layer and preserve root cause chain.

#### Scenario: Domain and infra errors are mapped without swallowing

- **WHEN** errors occur in domain logic or infrastructure adapters
- **THEN** they are represented as `DomainError` and `InfraError`
- **AND** mapped to `BridgeError` without semantic rewrite
- **AND** root cause chain remains traceable

#### Scenario: String flattening is prohibited

- **WHEN** bridge constructs outward-facing error payloads
- **THEN** it does not flatten all errors with `.to_string()` only
- **AND** contextual cause information is retained

### Requirement: Bridge Option Semantics and Anti-Fake-Object Safety

The bridge layer SHALL preserve Option semantics and reject fake placeholder object fallback.

#### Scenario: None is propagated correctly

- **WHEN** underlying lookup returns not found
- **THEN** service returns `Ok(None)`
- **AND** bridge payload reflects true null/none semantics

#### Scenario: Fake object fallback is prevented

- **WHEN** lookup result is absent
- **THEN** bridge must not return an object with empty-string fields as fallback
- **AND** negative tests verify the anti-fake-object guarantee
