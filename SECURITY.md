# Input and output boundaries

The renderer processes local chart JSON/gzip, images, skin manifests and optional
masterdata snapshots. Limits cap chart input, page dimensions, pixel surfaces and
encoded output. Source geometry and C parsing retain their documented bounds.
These checks are not a claim of complete security auditing or fuzz coverage of
Skia, font codecs or every third-party dependency.

Use trusted skin packs and a private output directory. The CLI refuses path
traversal in pack assets, symlink publication targets, protected-input overwrite
and concurrent publication to one group. Atomic snapshot readers use the
immutable-generation/current.json protocol documented in docs/OUTPUT.md.

Masterdata sync makes HTTPS requests only to the designated public GitHub
repository and validates downloaded tables before advancing the current pointer.
A corrupt or partial cache never replaces a valid current snapshot. The process
needs no account credentials. Credentials must not be placed in this repository,
command examples, screenshots or benchmark artifacts.

Dependency versions are locked. Native Windows/macOS CI results are separate
from Linux validation; do not assume cross-platform equivalence from a local run.
