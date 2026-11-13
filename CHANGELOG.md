# Changelog

## v0.2.0 - 2026/11/13

### Added

- `UploadUniform` implementation for `MeshId` for `uint` GLSL type

- Raycasting & AABB spatial look-up functions for `SpatialHash` types

- Pause/start toggle functionality for profiler

- Transient lifetime-bound mapping (non persistent/coherent) for `PartitionedBuffer`s abstractions

### Changed

- Update partitioned buffer abstractions to use named buffers according to DSA

### Fixed

- Correct `Column` indices de-sync over `free()` calls

## v0.1.0 - 2026/10/30

Base pre-changelog state
