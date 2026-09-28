[FJA-001] Use TOML as freyja build spec. Why? Because YAML sucks. [Read here](https://ruuda.nl/2023/the-yaml-document-from-hell)

[FJA-002] CLI is divided with functional part. So there's `freyja` crate for the CLI part.

[FJA-003] Extensions packs in separate crate by its `type` (for example `crates/freyja-extension-oci`)

[FJA-004] If extension provides new type of dependency, it should implement core's resolver trait.

[FJA-005] Builders engines should be in separate crates. Every builder should complain the core's builder trait

[FJA-006] Every key component/protocol/interface in `freyja-core` should be tested with "fake" implementation of it.

[FJA-007] Direct Buildkit communication temporaraly replaced by `podman buildx` cli interface

[FJA-008] For repo-dependent extensions(for example alt-rpm) in dependency resolve is local+cached pkglist and alternatives.
