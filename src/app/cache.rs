//! Bounded caches owned by the main thread.

use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::{Arc, OnceLock};

use lru::LruCache;

use crate::git::show::CommitDetails;
use crate::highlight::HighlightOutcome;
use crate::model::{BlameData, BlameKey, BlobSource, ContentKey, DiffKey, FileContent, FileDiff, Oid};
use crate::worker::HighlightKey;

const CONTENT_BYTES: usize = 256 * 1024 * 1024;

fn nz(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("non-zero")
}

pub fn absent() -> Arc<FileContent> {
    static ABSENT: OnceLock<Arc<FileContent>> = OnceLock::new();
    ABSENT.get_or_init(|| Arc::new(FileContent::Absent)).clone()
}

pub struct Caches {
    pub source_keys: HashMap<BlobSource, ContentKey>,
    pub content_errors: HashMap<BlobSource, String>,
    content: LruCache<ContentKey, Arc<FileContent>>,
    content_bytes: usize,
    pub highlight: LruCache<HighlightKey, HighlightOutcome>,
    pub diff: LruCache<DiffKey, Arc<FileDiff>>,
    pub blame: LruCache<BlameKey, Result<Arc<BlameData>, String>>,
    pub details: LruCache<Oid, Result<Arc<CommitDetails>, String>>,
}

impl Default for Caches {
    fn default() -> Self {
        Caches {
            source_keys: HashMap::new(),
            content_errors: HashMap::new(),
            content: LruCache::new(nz(256)),
            content_bytes: 0,
            highlight: LruCache::new(nz(64)),
            diff: LruCache::new(nz(128)),
            blame: LruCache::new(nz(32)),
            details: LruCache::new(nz(128)),
        }
    }
}

impl Caches {
    pub fn key_of(&self, src: &BlobSource) -> Option<ContentKey> {
        match src {
            BlobSource::Absent => Some(ContentKey::Absent),
            _ => self.source_keys.get(src).cloned(),
        }
    }

    pub fn content_by_key(&self, key: &ContentKey) -> Option<Arc<FileContent>> {
        match key {
            ContentKey::Absent => Some(absent()),
            _ => self.content.peek(key).cloned(),
        }
    }

    /// Ok(None) = not loaded yet.
    pub fn content_for(&self, src: &BlobSource) -> Result<Option<(ContentKey, Arc<FileContent>)>, String> {
        if let Some(e) = self.content_errors.get(src) {
            return Err(e.clone());
        }
        Ok(self.key_of(src).and_then(|k| self.content_by_key(&k).map(|c| (k, c))))
    }

    pub fn insert_content(&mut self, src: BlobSource, key: ContentKey, c: Arc<FileContent>) {
        self.content_errors.remove(&src);
        self.source_keys.insert(src, key.clone());
        if key == ContentKey::Absent {
            return;
        }
        self.content_bytes += c.approx_size();
        if let Some((_, old)) = self.content.push(key, c) {
            self.content_bytes -= old.approx_size();
        }
        while self.content_bytes > CONTENT_BYTES && self.content.len() > 1 {
            if let Some((_, old)) = self.content.pop_lru() {
                self.content_bytes -= old.approx_size();
            }
        }
    }

    /// Mark keys as recently used so the displayed data is never evicted.
    pub fn touch(&mut self, keys: &[ContentKey]) {
        for k in keys {
            self.content.promote(k);
        }
    }

    /// Reload: working-tree content may have changed, errors may be gone.
    pub fn invalidate_local(&mut self) {
        self.source_keys.retain(|s, _| !matches!(s, BlobSource::WorkingTree(_)));
        self.content_errors.clear();
        let failed: Vec<_> = self.blame.iter().filter(|(_, v)| v.is_err()).map(|(k, _)| k.clone()).collect();
        for k in failed {
            self.blame.pop(&k);
        }
        let failed: Vec<_> = self.details.iter().filter(|(_, v)| v.is_err()).map(|(k, _)| k.clone()).collect();
        for k in failed {
            self.details.pop(&k);
        }
    }
}
