use std::fs::File;
use std::path::Path;
use memmap2::Mmap;

/// Load a file using memory mapping.
pub fn load_with_memory_mapping(path: impl AsRef<Path>) -> anyhow::Result<Mmap> {
    let file = File::open(&path)?;
    unsafe { Mmap::map(&file) }.map_err(|e| e.into())
}