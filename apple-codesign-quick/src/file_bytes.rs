use crate::error::{CodeSignError, Result};
/* The whole point of isideload_vfs is to abstract away the difference
between the virtual file system and the real one, but in this case,
the Mmap crate only works with real files, so we have to use std::fs here. */
#[cfg(target_arch = "wasm32")]
use isideload_vfs::fs;
#[cfg(not(target_arch = "wasm32"))]
use memmap2::{Mmap, MmapOptions};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
use std::path::Path;

pub(crate) enum FileBytes {
    #[cfg(not(target_arch = "wasm32"))]
    Mapped(Mmap),
    Owned(Vec<u8>),
}

impl FileBytes {
    pub(crate) fn as_slice(&self) -> &[u8] {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Mapped(map) => map,
            Self::Owned(bytes) => bytes,
        }
    }
}

pub(crate) fn read_file_bytes(path: &Path) -> Result<FileBytes> {
    let file = fs::File::open(path).map_err(|source| CodeSignError::io(path, source))?;
    let len = file
        .metadata()
        .map_err(|source| CodeSignError::io(path, source))?
        .len();

    if len == 0 {
        return Ok(FileBytes::Owned(Vec::new()));
    }

    // SAFETY: every mapped read in this crate is created from a file opened for
    // read-only use and is only exposed as an immutable byte slice. During our
    // own signing flow, files are mapped after any planned in-process rewrites
    // for that file and before we write the signed replacement. Concurrent
    // external mutation of the bundle while signing is outside the supported
    // contract, matching the same precondition as the D MmFile implementation.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mapped = unsafe { MmapOptions::new().map(&file) };
        match mapped {
            Ok(map) => Ok(FileBytes::Mapped(map)),
            Err(_) => fs::read(path)
                .map(FileBytes::Owned)
                .map_err(|source| CodeSignError::io(path, source)),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        fs::read(path)
            .map(FileBytes::Owned)
            .map_err(|source| CodeSignError::io(path, source))
    }
}
