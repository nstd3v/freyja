- [FB-001] **OCI pin delivery.** 
  Extend the builder interface so it receives the exact resolved dependency map from the plan (not a second resolution). 
  For mapped OCI dependencies, pass a digest-pinned image reference through the declared build argument. 
  Validate each mapping and ensure a changed digest changes the command input and the stored state. 
  Use a fake `podman` executable that records argv; assert the argument contains the digest and no credentials. 
  Update the nginx example to use `ARG`/`FROM` for its OCI base while retaining the now-registered ALT RPM dependency. 
  Do not claim reproducible/pinned OCI builds before this is verified.
- [FB-002] **Resolve context hashing according to ignore files.**
  Define and test how `.dockerignore`/`.containerignore` affect the fingerprint without missing files Podman can consume. The current conservative full-context hash may cause unnecessary rebuilds.
- [FB-003] **Honor declared tags and architectures.**
  The target model and fingerprint include `tags` and `arches`, but the Podman invocation currently uses only `image`, Dockerfile, and context. Wire supported options into the build command and test its argv, or reject unsupported values rather than rebuilding without applying them.
- [FB-005] **Keep README aligned with implemented behavior.**
  ALT RPM and APK registration, the active builder crate, rebuild triggers versus pinned build inputs, and the `oci.enabled`, `alt_rpm.enabled`, and `apk.enabled` switches are documented. Keep the README current as supported target options and remaining extension settings are implemented.
- [FB-006] **Deliver resolved APK version to the build.**
  The `apk` resolver compares version and checksum to schedule builds, but `apk add` currently uses repository state at build time. Extend the builder interface and Dockerfile contract so the version resolved by the plan is applied to `apk add`, test its actual command input, and avoid claiming pinned APK builds before this is verified.
- [FB-007] **Verify APKINDEX authenticity.**
  The APK resolver fetches official indexes over HTTPS but does not validate the archive signature. Define trusted Alpine keys, implement signature verification before caching/resolving, and test tampering and key rotation.
- [FB-008] **Prevent duplicate effective image tags across targets.**
  The builder tags each target with `target.image`, while state is tracked separately by target. Two targets with the same image reference can both record success even though the second build replaces the first image. Reject collisions before building (including any additional tags introduced by FB-003), or model the shared output explicitly; test that a conflicting spec cannot produce a misleading `SKIP`.
- [FB-009] **Include build-relevant file modes in context fingerprints.**
  `fingerprint.rs` hashes paths, lengths, and bytes but not executable bits. A `0644` to `0755` change on a copied file can leave `plan` at `SKIP`. Hash relevant permissions and test mode-only changes on Unix without treating irrelevant metadata such as mtime as build inputs.
- [FB-010] **Create state temporary files safely.**
  `State::save` writes to the predictable `<state>.tmp` path, following a pre-existing symlink and potentially overwriting an unrelated writable file. Use a unique, exclusively created temporary file in the state directory and atomic replacement; test symlink refusal and cleanup on failure.
- [FB-011] **Coordinate concurrent state writers.**
  Separate `build` processes can load the same state and overwrite each other's records through the shared temporary path. Lock the state transaction or detect conflicts and merge safely; test simultaneous builds of different targets so both records survive and neither command reports false success.
- [FB-012] **Validate ALT RPM indexes before caching.**
  The ALT RPM resolver writes downloaded compressed bytes before decompression and package parsing. A malformed response is then reused as a fresh cache entry until expiry. Parse and validate before atomic cache replacement; test that corrupt downloads do not poison a previous valid cache.
- [FB-013] **Expire in-memory package indexes.**
  APK and ALT RPM resolvers check the disk TTL only on the first lookup for a repository/architecture key. In a long-lived process, the in-memory map can keep serving old metadata indefinitely. Track freshness or revalidate on subsequent lookups; test a clock/cache transition without relying on live network.
- [FB-014] **Bound ALT RPM FTP work.**
  FTP connection, login, transfer, and completion have no overall deadline, while compressed download and expanded XZ parsing have no size caps. Apply timeouts and compressed/decompressed limits with clear errors; test stalled streams and oversized payloads without allocating unbounded memory.
