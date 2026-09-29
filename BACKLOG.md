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
- [FB-004] **Make the nginx example runnable with the active workspace.**
  ALT RPM is now registered, but the example's Dockerfile still uses a mutable base tag and there has been no real registry/FTP/Podman end-to-end run. As part of FB-001, consume a digest-pinned base image through `ARG`/`FROM`; then verify the example's plan/build path with live services. Do not remove its ALT RPM dependency solely because the old resolver was disabled.
- [FB-005] **Align README with implemented behavior.**
  Partly addressed: ALT RPM and APK are registered, the active builder crate name is corrected, and the README distinguishes rebuild triggers from pinned build inputs. Finish by documenting actual extension-setting behavior and supported target options as they are implemented.
- [FB-006] **Deliver resolved APK version to the build.**
  The `apk` resolver compares version and checksum to schedule builds, but `apk add` currently uses repository state at build time. Extend the builder interface and Dockerfile contract so the version resolved by the plan is applied to `apk add`, test its actual command input, and avoid claiming pinned APK builds before this is verified.
- [FB-007] **Verify APKINDEX authenticity.**
  The APK resolver fetches official indexes over HTTPS but does not validate the archive signature. Define trusted Alpine keys, implement signature verification before caching/resolving, and test tampering and key rotation.
