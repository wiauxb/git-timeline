# `src/git/blob.rs`

## Purpose
Read the bytes of a version, whatever its `BlobSource`, and classify them into a
`FileContent`. Uses a **persistent `git cat-file --batch-command --buffer` process** per
reader (one reader per worker thread) instead of one process per blob.

## Contents

```rust
pub struct BlobReader { git: Git, batch: Option<CatFileBatch>, limit: u64 }

impl BlobReader {
    pub fn new(git: Git) -> Self;                       // limit = MAX_BLOB_SIZE
    #[doc(hidden)] pub fn with_limit(git: Git, limit: u64) -> Self;   // tests
    /// Content and the cache key it belongs under.
    pub fn read(&mut self, src: &BlobSource, mode: FileMode) -> Result<(ContentKey, FileContent)>;
    /// Unresolved → (Object(oid), mode) or (Absent, Absent).
    pub fn resolve(&mut self, commit: &Oid, path: &RepoPath) -> Result<(BlobSource, FileMode)>;
}

struct CatFileBatch { child: Child, stdin: BufWriter<ChildStdin>, stdout: BufReader<ChildStdout> }
impl CatFileBatch {
    fn spawn(git: &Git) -> Result<Self>;              // stderr → null
    fn info(&mut self, oid: &Oid) -> Result<Option<u64>>;   // "info <oid>\nflush\n"
    fn contents(&mut self, oid: &Oid) -> Result<Vec<u8>>;   // "contents <oid>\nflush\n"
}
impl Drop for CatFileBatch { /* kill + wait */ }
```

## Per source
* `Absent` ⇒ `(ContentKey::Absent, Absent)`.
* `Object(oid)` — key `ContentKey::Object(oid)` (identical blobs in different commits share
  one cache entry):
  1. `mode == Submodule` ⇒ `Submodule{commit: oid}` (no read).
  2. `info` ⇒ size; `missing` ⇒ `Error::Parse("missing object")`; `> limit` ⇒ `TooLarge`.
  3. `contents` ⇒ `content::classify(bytes, mode)`.
* `WorkingTree(path)`: `symlink_metadata` (not following). NotFound ⇒ Absent. Key
  `WorkingTree{path, mtime_ns, len}`. Mode from `status::mode_of`: symlink ⇒ `read_link` ⇒
  `Symlink{target}`; `len > limit` ⇒ `TooLarge`; else read + classify.
* `Unresolved{commit, path}`: `git ls-tree -z <commit> -- <path>` (gives oid **and** mode),
  then read as `Object` with that mode (the caller's mode is ignored).

## Batch protocol (verified)
`info <oid>` → `<oid> <type> <size>\n` or `<oid> missing\n`.
`contents <oid>` → header line, `<size>` bytes, then `\n`.
With `--buffer` each command must be followed by `flush`. If an `Io` error occurs (process
died), the process is restarted and the operation retried once.

## Known limitation
Working-tree bytes are read **raw**: no `.gitattributes` filters (CRLF normalisation, LFS,
smudge). With `core.autocrlf`, the working-tree row can look entirely changed vs the index.
Candidate fix: `git hash-object --path=<p> --stdin` then read back the cleaned blob.

## Tests (tests/views.rs)
Content equals `git show`; binary / too-large (lowered limit) / symlink classification.
