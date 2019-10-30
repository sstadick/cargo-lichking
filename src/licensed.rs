use cargo::core::Package;

use crate::license::License;

pub trait Licensed {
    fn license(&self) -> License;
}

impl Licensed for Package {
    fn license(&self) -> License {
        let metadata = self.manifest().metadata();
        metadata
            .license
            .as_ref()
            .and_then(|license| license.parse::<License>().ok())
            .or_else(|| {
                metadata
                    .license_file
                    .as_ref()
                    .and_then(|file| self.root().join(file).canonicalize().ok())
                    .map(License::File)
            })
            .unwrap_or_default()
    }
}
