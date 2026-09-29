# freyja

Freyja is a local-first, dependency-aware OCI image build orchestrator that rebuilds an image only when one of its declared dependencies changes.

> **WIP** -- project right now in active development

## Prerequisites

- Rust stable (the workspace uses edition 2024)
- podman with buildx support — builds are run via `podman buildx`
- Network access to OCI registries and ALT mirrors (FTP)

## Build

```sh
cargo build --release -p freyja
```

The binary is named `freyja`.

## Quick start

From the [`examples/nginx-alt/`](./examples/nginx-alt/) directory (requires access to the OCI registry and ALT FTP mirror):

```sh
freyja --state ../.freyja/state.toml plan           # show which targets need to be rebuilt
freyja --state ../.freyja/state.toml build          # build the targets that need it
freyja --state ../.freyja/state.toml explain nginx  # explain why a target needs to be rebuilt
```

## CLI reference

Global flags (accepted by all subcommands):

| Flag | Default | Description |
| --- | --- | --- |
| `-d, --dir <DIR>` | `.freyja` | Freyja dir |
| `-f, --file <FILE>` | `freyja.toml` | Path to the Freyja configuration file |
| `-s, --state <STATE>` | `.freyja/state.toml` | Path to the state file |

Subcommands:

- `plan` — show which targets need to be rebuilt
- `build` — build targets that need to be rebuilt
- `explain <target>` — explain why a target needs to be rebuilt

## `freyja.toml` spec

Top level:

| Field | Description |
| --- | --- |
| `version` | Spec version (string) |
| `title` | Human-readable project title |
| `[extensions]` | Which dependency extensions are enabled |
| `[targets.<name>]` | One entry per image to build |

Extensions:

| Field | Description |
| --- | --- |
| `oci.enabled` | Enable the `oci` dependency type |
| `oci.registry` | Optional registry override |
| `alt_rpm.enabled` | Enable the `alt_rpm` dependency type |
| `alt_rpm.repository` | Optional repository override |

Note: extension settings are parsed but not yet enforced — both resolvers are always registered. ALT RPM currently supports the built-in `sisyphus` repository over FTP; HTTP repositories are not implemented. Resolved OCI digests and ALT RPM versions trigger rebuild decisions but are not pinned into the Podman build.

Target fields (`[targets.<name>]`):

| Field | Description |
| --- | --- |
| `image` | Image reference to build and tag |
| `tags` | Tags to apply to the image |
| `arches` | Architectures to build: `amd64`, `arm64` |
| `build.context` | Build context path, resolved relative to the spec file's directory |
| `build.dockerfile` | Dockerfile inside the context (default `Dockerfile`); passed to Podman with `-f` |
| `[targets.<name>.dependencies.<dep>]` | Declared dependencies of the target |

Every dependency has a `type` plus type-specific fields.

`oci`:

| Field | Description |
| --- | --- |
| `ref` | Image reference to pin (resolved to a manifest digest) |

`alt_rpm`:

| Field | Description |
| --- | --- |
| `repository` | ALT repository name; the built-in one is `sisyphus` (`ftp.altlinux.org:/pub/distributions/ALTLinux/Sisyphus`) |
| `arch` | Package architecture, e.g. `x86_64`, `aarch64` |
| `package` | Package name to resolve |

See [`examples/nginx-alt/freyja.toml`](./examples/nginx-alt/freyja.toml) for a concrete reference.

## State & cache

- The state file stores resolved-dependency and build-input fingerprints per target (saved atomically). `plan` compares them to decide `BUILD` or `SKIP`; old state without a build-input fingerprint triggers one rebuild. Keep the state file **outside every build context** (use `--state`): Freyja rejects a state path inside a context to prevent a rebuild loop.
- Build-input fingerprints conservatively hash all regular files and directory paths in the context, including files ignored by Podman and generated files. Keep contexts small; Git/Cargo ignores do not limit this hash. Symlinks and special files in the context are currently rejected rather than silently skipped.
- `.freyja/cache/alt-rpm/` — caches ALT package lists fetched from mirrors, so repeated runs do not re-download them within the cache TTL (1 hour).

## Project layout

| Crate | Description |
| --- | --- |
| [`crates/freyja`](./crates/freyja) | CLI: parses arguments and wires up extensions, planner and builder |
| [`crates/freyja-core`](./crates/freyja-core) | Spec model, resolver registry, planner, state; `DependencyResolver` and `Builder` traits |
| [`crates/freyja-extension-oci`](./crates/freyja-extension-oci) | Resolves `oci` dependencies (image reference to manifest digest) |
| [`crates/freyja-extension-altrpm`](./crates/freyja-extension-altrpm) | Resolves `alt_rpm` dependencies from ALT package lists over FTP |
| [`crates/freyja-builder-podman`](./crates/freyja-builder-podman) | Implements `Builder`; shells out to `podman buildx` |

## Extending

To add a new dependency type or builder engine, implement the core traits (`DependencyResolver`, `Builder`) in a new crate and register it with the CLI. See [CHOICE.md](./CHOICE.md) for the design rationale.

## Examples

- [`examples/nginx-alt/`](./examples/nginx-alt/) — an nginx image depending on an ALT base image (`oci`) and the `nginx` package (`alt_rpm`).

## License

[MIT](./LICENSE).

## Contribute

Pull requests are welcome — docs, new extensions, or anything else.
