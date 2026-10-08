# `src/git/blob.rs`

## Purpose
Read the bytes of a version, whatever its `BlobSource`, and classify them into a
`FileContent`. Uses a **persistent `git cat-file --batch-command` process** per worker
thread to avoid one process spawn per scroll step.

## Contents

```rust
pub struct BlobReader {
    git: Git,
    batch: Option<CatFileBatch>,      // lazily started, restarted if it dies
}

impl BlobReader {
    pub fn new(git: Git) -> Self;
    /// Returns the content and the cache key it should be stored under.
    pub fn read(&mut self, src: &BlobSource, mode: FileMode, toplevel: &Path)
        -> Result<(ContentKey, FileContent)>;
    /// Resolve `BlobSource::Unresolved` → Object / Absent.
    pub fn resolve(&mut self, commit: &Oid, path: &RepoPath) -> Result<BlobSource>;
}

struct CatFileBatch { child: Child, stdin: BufWriter<ChildStdin>, stdout: BufReader<ChildStdout> }
impl CatFileBatch {
    fn spawn(git: &Git) -> Result<Self>;          // git cat-file --batch-command --buffer
    fn info(&mut self, oid: &Oid) -> Result<Option<u64>>;   // "info <oid>" + flush
    fn contents(&mut self, oid: &Oid) -> Result<Vec<u8>>;   // "contents <oid>" + flush
}
```

## Per source
* `Object(oid)`:
  1. `mode == Submodule` ⇒ `FileContent::Submodule{commit: oid}` (no read; it's a commit
     id of another repo).
  2. `info` → size; `> MAX_BLOB_SIZE` ⇒ `TooLarge` (no read).
  3. `contents` → bytes → `content::classify(bytes, mode)` (symlink ⇒ `Symlink{target}`).
  Key: `ContentKey::Object(oid)` — **identical blobs in different commits share one cache
  entry**.
* `WorkingTree(path)`: `std::fs::symlink_metadata` (no following) → mtime/len;
  symlink ⇒ `read_link`; else read file (size-limited). Missing ⇒ `Absent`.
  Key: `ContentKey::WorkingTree{path, mtime_ns, len}`.
* `Absent` ⇒ `FileContent::Absent`, key `ContentKey::Absent`.
* `Unresolved{commit, path}`: `git rev-parse --verify -q <commit>:<path>` (+ `ls-tree` for
  the mode) ⇒ then read as `Object`. Missing ⇒ `Absent`.

## Batch protocol
`info <oid>\n` → `<oid> <type> <size>\n` or `<oid> missing\n`.
`contents <oid>\n` → `<oid> <type> <size>\n<bytes>\n`.
`--buffer` requires an explicit `flush\n` after each command. One reader per worker
thread ⇒ no locking. If the process dies, restart it once, then surface the error.

## Known limitation
Working-tree bytes are read **raw**: no `.gitattributes` clean filters (CRLF
normalisation, LFS, smudge). In repos relying on `core.autocrlf`, the working-tree row can
look entirely changed vs. the index. Documented in README; fix candidate for v2:
`git hash-object --path=<p> -w --stdin` then read back the cleaned blob.
