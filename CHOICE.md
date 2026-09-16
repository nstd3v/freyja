<FJA-001> Use TOML as freyja build spec. Why? Because YAML sucks. Read here: https://ruuda.nl/2023/the-yaml-document-from-hell
<FJA-002> CLI is divided with functional part. So there's `freyja` crate for the CLI part.
<FJA-003> Extensions packs in separate crate by its `type` (for example `crates/freyja-extension-oci`)
<FJA-004> If extension provides new type of dependency, it should implement core's resolver trait.
