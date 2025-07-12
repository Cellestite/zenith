use std::hash::Hasher;
use foldhash::fast::FoldHasher;
use foldhash::SharedSeed;

pub type SmallVec<A> = smallvec::SmallVec<A>;

pub type HashMap<K, V> = hashbrown::HashMap<K, V>;
pub type Entry<'a, K, V, S, A> = hashbrown::hash_map::Entry<'a, K, V, S, A>;
pub type HashSet<K, V> = hashbrown::HashSet<K, V>;

pub struct DefaultHasher(FoldHasher);

impl DefaultHasher {
    pub fn new() -> Self {
        Self(FoldHasher::with_seed(0, SharedSeed::global_random()))
    }
}

impl Hasher for DefaultHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
    
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes);
    }
}