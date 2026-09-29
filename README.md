# freyja

Freyja is a local-first, dependency-aware OCI image build orchestrator. It plans builds from resolved dependencies and build-context inputs, then runs the builds through Podman.

> **WIP** -- project right now in active development

## Prerequisites

- Rust stable (the workspace uses edition 2024)
- podman with buildx support — builds are run via `podman buildx`
- Network access to OCI registries and the ALT FTP mirror when using those dependency types

## Build

```sh
cargo build --release -p freyja
```

The binary is named `freyja`.

## Quick start

From the repository root, after building the binary (requires access to the OCI registry and ALT FTP mirror):

```sh
target/release/freyja --file examples/nginx-alt/freyja.toml --state .freyja/state.toml plan
target/release/freyja --file examples/nginx-alt/freyja.toml --state .freyja/state.toml explain nginx
target/release/freyja --file examples/nginx-alt/freyja.toml --state .freyja/state.toml build
```

`plan` and `explain` resolve dependencies (and may fetch metadata); they do not build an image. `build` saves state after each successful target build. The example's image name uses `registry.example.com` as a placeholder: change it before using the resulting tag or publishing anything. This example has not been verified with live registry, FTP, and Podman services end to end.

## CLI reference

Global flags (accepted by all subcommands):

| Flag | Default | Description |
| --- | --- | --- |
| `-d, --dir <DIR>` | `.freyja` | Directory for the ALT RPM cache |
| `-f, --file <FILE>` | `freyja.toml` | Path to the Freyja configuration file |
| `-s, --state <STATE>` | `.freyja/state.toml` | Path to the state file |

Relative `--file`, `--dir`, and `--state` paths are interpreted from the current working directory; build contexts in the spec are resolved relative to the spec file. The default state path must be outside every build context; if the context is the current directory, pass an external `--state` path.

Subcommands:

- `plan` — show which targets need to be rebuilt
- `build` — build targets that need to be rebuilt
- `explain <target>` — explain why a target needs to be rebuilt

## `freyja.toml` spec

Top level:

| Field | Description |
| --- | --- |
| `version` | Spec version (string; the example uses `"1"`, distinct from the CLI package version) |
| `title` | Human-readable project title |
| `[extensions]` | Currently an inert table; does not control resolver registration |
| `[targets.<name>]` | One entry per image to build |

The CLI always registers both `oci` and `alt_rpm` resolvers. Entries such as `oci.enabled`, `oci.registry`, `alt_rpm.enabled`, and `alt_rpm.repository` in `[extensions]` are currently ignored; they neither enable/disable resolvers nor override registries/repositories. ALT RPM currently supports the built-in `sisyphus` repository over FTP; HTTP repositories are not implemented.

Target fields (`[targets.<name>]`):

| Field | Description |
| --- | --- |
| `image` | Image reference to build and tag |
| `tags` | Required list of tags; parsed and fingerprinted, **not applied** by the current builder |
| `arches` | Required list of architectures (`amd64`, `arm64`); parsed and fingerprinted, **not passed** to the current builder |
| `build.context` | Build context path, resolved relative to the spec file's directory |
| `build.dockerfile` | Dockerfile inside the context (default `Dockerfile`); passed to Podman with `-f` |
| `[targets.<name>.dependencies.<dep>]` | Declared dependencies of the target |

Every dependency has a `type` plus type-specific fields. The current Podman command uses the context, Dockerfile and `image` (`podman buildx build -f <dockerfile> <context> -t <image> --load`); it does not push the image. `tags` and `arches` can trigger a rebuild when changed, but do not affect the command. Resolved dependency values are compared to prior state, **not injected or pinned into the Dockerfile/build**: the example's `FROM ...:latest` remains mutable even when its `oci` dependency resolves to a digest. Do not rely on this release for reproducible or digest-pinned builds.

`oci`:

| Field | Description |
| --- | --- |
| `ref` | Image reference whose manifest digest is checked anonymously for changes (not pinned in the build) |

`alt_rpm`:

| Field | Description |
| --- | --- |
| `repository` | ALT repository name; the built-in one is `sisyphus` (`ftp.altlinux.org:/pub/distributions/ALTLinux/Sisyphus`) |
| `arch` | Package architecture, e.g. `x86_64`, `aarch64` |
| `package` | Package name to resolve |

See [`examples/nginx-alt/freyja.toml`](./examples/nginx-alt/freyja.toml) for a concrete reference.

## State & cache

- The state file stores resolved-dependency and build-input fingerprints per target (saved via a temporary file and rename). `plan` compares them to decide `BUILD` or `SKIP`; old state without a build-input fingerprint triggers one rebuild. Keep the state file **outside every build context** (use `--state`): Freyja rejects a state path inside a context to prevent a rebuild loop. `SKIP` is a fingerprint decision, not a check that the locally tagged image still exists.
- Build-input fingerprints conservatively hash all regular files and directory paths in the context, including files ignored by Podman and generated files. Keep contexts small; Git/Cargo ignores do not limit this hash. Symlinks and special files in the context are currently rejected rather than silently skipped.
- `<DIR>/cache/alt-rpm/` (by default `.freyja/cache/alt-rpm/`) caches ALT package lists fetched from the mirror for up to 1 hour.

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

### Repositories
This project hosts on two platforms:
- [GitHub](https://github.com/nstd3v/freyja)
- [ALS](https://altlinux.space/stavtnr/freyja)

ALS is primary development repository and GitHub as release platform.
