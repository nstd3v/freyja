# freyja-extension-apk

### APK extension internals

The [`freyja-extension-apk` source](./crates/freyja-extension-apk/src/) separates resolver wiring from the index mechanics:

| Module | Responsibility |
| --- | --- |
| [`lib.rs`](./crates/freyja-extension-apk/src/lib.rs) | `ApkResolver` and its `DependencyResolver` implementation; converts a resolved package into fingerprint and metadata |
| [`dependency.rs`](./crates/freyja-extension-apk/src/dependency.rs) | `ApkDependency` fields, official-repository validation, and reference formatting |
| [`error.rs`](./crates/freyja-extension-apk/src/error.rs) | Typed `ApkError` failures for configuration, fetch, cache, index, and missing packages |
| [`cache.rs`](./crates/freyja-extension-apk/src/cache.rs) | Resolver constructor, HTTPS retrieval, bounded downloads, one-hour disk/in-memory cache, and index loading |
| [`index.rs`](./crates/freyja-extension-apk/src/index.rs) | Concatenated-gzip `APKINDEX.tar.gz` extraction and package-record parsing |

The extension's [`tests.rs`](./crates/freyja-extension-apk/src/tests.rs) covers index parsing, cached resolution, and changed-input planning; the CLI has an [`apk` routing test](./crates/freyja/tests/apk_cli.rs). The cache is not a package store, and neither these tests nor `plan` verify an image build.
