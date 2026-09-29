- [FB-001] **OCI pin delivery.** 
  Extend the builder interface so it receives the exact resolved dependency map from the plan (not a second resolution). 
  For mapped OCI dependencies, pass a digest-pinned image reference through the declared build argument. 
  Validate each mapping and ensure a changed digest changes the command input and the stored state. 
  Use a fake `podman` executable that records argv; assert the argument contains the digest and no credentials. 
  Update the nginx example to use `ARG`/`FROM` and remove its currently unsupported ALT RPM declaration for this slice. 
  Do not claim reproducible/pinned OCI builds before this is verified.
- [FB-002] **Resolve context hasing according ignore files**
