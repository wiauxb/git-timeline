# `src/app/cache.rs`

## Purpose
Bounded in-memory caches, owned by the main thread (`AppState::cache`). Makes moving
back and forth in history instant after the first visit.

## Contents

```rust
pub struct Caches {
    pub source_keys: HashMap<BlobSource, ContentKey>,          // unbounded, tiny
    pub content:   lru::LruCache<ContentKey, Arc<FileContent>>,
    pub highlight: lru::LruCache<(ContentKey, String), HighlightOutcome>,
    pub diff:      lru::LruCache<DiffKey, Arc<FileDiff>>,
    pub blame:     lru::LruCache<BlameKey, Result<Arc<BlameData>, Arc<Error>>>,
    pub details:   lru::LruCache<Oid, Result<Arc<CommitDetails>, Arc<Error>>>,
    content_bytes: usize,                                      // running total
}

impl Caches {
    pub fn new() -> Self;
    pub fn content_for(&self, src: &BlobSource) -> Option<&Arc<FileContent>>;
    pub fn insert_content(&mut self, src: BlobSource, key: ContentKey, c: Arc<FileContent>);
    // + get/insert for each cache
}
```

## Limits
| Cache | Bound |
|---|---|
| content | 256 entries **and** 256 MiB total bytes (evict LRU until under both) |
| highlight | 64 entries |
| diff | 128 entries |
| blame | 32 entries |
| details | 128 entries |

## Specs
* `BlobSource::Object` content never goes stale (content-addressed). Working-tree content
  is keyed with mtime+len, so a reload naturally misses the cache after an edit.
* The **content currently displayed** (and the diff/highlight it uses) is "pinned":
  `update.rs` re-touches those keys after each insert so they are never the LRU victim.
* Errors are cached too (blame/details) to avoid retry storms; `Reload` clears error
  entries.
