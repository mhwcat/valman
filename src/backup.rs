use std::{fs::File, path::Path};

use flate2::read::GzDecoder;
use log::debug;
use tar::Archive;

use crate::error::Result;

pub fn restore_backup(backup_path: &Path, destination_path: &Path) -> Result<()> {
    debug!(
        "Restoring backup {} to {}",
        backup_path.display(),
        destination_path.display()
    );

    let tar_gz = File::open(backup_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(destination_path)?;

    Ok(())
}
