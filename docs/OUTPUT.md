# Output groups and failure behavior

A CLI render completes parsing, layout and all PNG encoding **before** publication.
`output::publish` accepts an explicit set of basename/bytes pairs. A writer lock
prevents two processes publishing to the same group concurrently.

For `-o chart.png`, publication creates an immutable generation under
`chart.render-set/generation-.../`, containing the complete multi-column PNG, optional scene JSON
and render report. After files are flushed, `chart.render-set/current.json` is
replaced atomically. It lists generation, filenames, byte counts and SHA-256.
Readers requiring a consistent snapshot read this pointer and use only that
generation. This is the authoritative commit point.

Sibling `chart.png` and `chart.render.json` remain convenient aliases.
Legacy numbered outputs from an older manifest are cleaned up on replacement. They are rollback-protected for ordinary I/O errors and stale prior
page names are removed, but multiple loose files cannot be atomically renamed
as a set. A process/OS crash may interrupt alias updates; `current.json` still
selects the old complete generation. Re-running restores aliases. Do not claim
crash-atomic semantics for the loose filenames themselves.

Previous generations are retained intentionally for audit/rollback, and disk
usage grows with repeated writes. No unrelated directories are deleted. Remove
old generation directories only when no reader uses them. Protected input paths,
unsafe output basenames, symlink destinations and duplicate names are rejected.
Scene JSON must share the output directory to participate in the transaction.
