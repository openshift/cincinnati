//! Render a docker image.

// Docker image format is specified at
// https://github.com/moby/moby/blob/v17.05.0-ce/image/spec/v1.md

use libflate::gzip;
use std::{fs, io, path};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("wrong target path {}: must be absolute path to existing directory", _0.display())]
    WrongTargetPath(path::PathBuf),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

/// Unpack an ordered list of layers to a target directory.
///
/// Layers must be provided as gzip-compressed tar archives, with lower layers
/// coming first. Target directory must be an existing absolute path.
pub fn unpack(layers: &[Vec<u8>], target_dir: &path::Path) -> Result<(), RenderError> {
    _unpack(layers, target_dir, |mut archive, target_dir| {
        Ok(archive.unpack(target_dir)?)
    })
}

/// Unpack an ordered list of layers to a target directory, filtering
/// file entries by path.
///
/// Layers must be provided as gzip-compressed tar archives, with lower layers
/// coming first. Target directory must be an existing absolute path.
pub fn filter_unpack<P>(
    layers: &[Vec<u8>],
    target_dir: &path::Path,
    predicate: P,
) -> Result<(), RenderError>
where
    P: Fn(&path::Path) -> bool,
{
    _unpack(layers, target_dir, |mut archive, target_dir| {
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;

            if predicate(&path) {
                entry.unpack_in(target_dir)?;
            }
        }

        Ok(())
    })
}

fn _unpack<U>(layers: &[Vec<u8>], target_dir: &path::Path, unpacker: U) -> Result<(), RenderError>
where
    U: Fn(tar::Archive<gzip::Decoder<&[u8]>>, &path::Path) -> Result<(), RenderError>,
{
    if !target_dir.is_absolute() || !target_dir.exists() || !target_dir.is_dir() {
        return Err(RenderError::WrongTargetPath(target_dir.to_path_buf()));
    }
    for l in layers {
        // Unpack layers
        let gz_dec = gzip::Decoder::new(l.as_slice())?;
        let mut archive = tar::Archive::new(gz_dec);
        archive.set_preserve_permissions(true);
        archive.set_unpack_xattrs(true);
        unpacker(archive, target_dir)?;

        // Clean whiteouts
        let gz_dec = gzip::Decoder::new(l.as_slice())?;
        let mut archive = tar::Archive::new(gz_dec);
        for entry in archive.entries()? {
            let file = entry?;
            let path = file.path()?;
            let parent = path.parent().unwrap_or_else(|| path::Path::new("/"));
            if let Some(fname) = path.file_name() {
                let wh_name = fname.to_string_lossy();
                if wh_name == ".wh..wh..opq" {
                    //TODO(lucab): opaque whiteout, dir removal
                } else if wh_name.starts_with(".wh.") {
                    let rel_parent =
                        path::PathBuf::from("./".to_string() + &parent.to_string_lossy());

                    // Remove real file behind whiteout
                    let real_name = wh_name.trim_start_matches(".wh.");
                    let abs_real_path = target_dir.join(&rel_parent).join(real_name);
                    remove_whiteout(abs_real_path)?;

                    // Remove whiteout place-holder
                    let abs_wh_path = target_dir.join(&rel_parent).join(fname);
                    remove_whiteout(abs_wh_path)?;
                };
            }
        }
    }
    Ok(())
}

// Whiteout files in archive may not exist on filesystem if they were
// filtered out via filter_unpack.  If not found, that's ok and the
// error is non-fatal.  Otherwise still return error for other
// failures.
fn remove_whiteout(path: path::PathBuf) -> io::Result<()> {
    let res = fs::remove_dir_all(path);

    match res {
        Ok(_) => res,
        Err(ref e) => match e.kind() {
            io::ErrorKind::NotFound => Ok(()),
            _ => res,
        },
    }
}
