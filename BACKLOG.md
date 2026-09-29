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
  Partly addressed: ALT RPM is registered, the active builder crate name is corrected, and the README distinguishes rebuild triggers from pinned build inputs. Finish by documenting actual extension-setting behavior and supported target options as they are implemented.
