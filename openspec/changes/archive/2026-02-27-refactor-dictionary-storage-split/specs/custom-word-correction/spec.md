## REMOVED Requirements
### Requirement: Legacy Dictionary Migration Compatibility
**Reason**: Dictionary storage is no longer embedded in settings, and this pre-production split adopts a hard-reset policy with no legacy dictionary import.
**Migration**: None. Existing legacy `settings.dictionary` values are intentionally not imported into the new dictionary file.

## ADDED Requirements
### Requirement: Independent Dictionary Persistence
The system SHALL persist dictionary entries in a dedicated app-data file independent of `AppSettings` payload storage.

#### Scenario: Dictionary persisted outside settings
- **WHEN** dictionary entries are saved
- **THEN** they are written to `user_dictionary.json` under app data
- **AND** `get_app_settings` payload does not include a `dictionary` field

### Requirement: Runtime Dictionary State Uses In-Memory Snapshot
The system SHALL load dictionary entries into managed in-memory state at startup and runtime consumers SHALL read from that in-memory snapshot.

#### Scenario: Hot-path consumers avoid per-request file I/O
- **WHEN** transcription or correction logic needs dictionary entries
- **THEN** it reads from managed in-memory dictionary state
- **AND** it does not perform dictionary file reads on each request

### Requirement: Serialized Disk-First Dictionary Writes
The system SHALL serialize dictionary updates and apply a disk-write-first consistency policy.

#### Scenario: Successful write swaps memory after disk
- **WHEN** `set_user_dictionary` is called
- **THEN** the write operation is serialized behind a dedicated write gate
- **AND** dictionary file persistence completes before in-memory state is swapped

#### Scenario: Disk write failure preserves memory
- **WHEN** dictionary file persistence fails
- **THEN** the command returns an error
- **AND** in-memory dictionary state remains unchanged

### Requirement: Dictionary File Version Fallback
The system SHALL accept dictionary envelope version `1` and treat other versions as unsupported.

#### Scenario: Unsupported dictionary version fallback
- **WHEN** dictionary file version is not `1`
- **THEN** the system logs a warning with diagnostic context
- **AND** initializes runtime dictionary state as empty

### Requirement: Settings Reset Does Not Affect Dictionary
Resetting settings SHALL not reset or mutate dictionary data.

#### Scenario: Reset settings keeps dictionary
- **WHEN** `reset_app_settings` is executed
- **THEN** settings payload is reset to defaults
- **AND** `user_dictionary.json` remains unchanged

### Requirement: Legacy Settings Dictionary Is Not Imported
The system SHALL not import legacy dictionary values from settings payload during startup or runtime.

#### Scenario: Legacy settings dictionary remains ignored
- **WHEN** settings storage still contains legacy `settings.dictionary` bytes
- **THEN** dictionary runtime state is sourced only from `user_dictionary.json`
- **AND** legacy settings dictionary values are ignored

