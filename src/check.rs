use anyhow::anyhow;
use cargo_metadata::Package;

use crate::licensed::Licensed;

pub fn run(root: &Package, packages: &[&Package]) -> anyhow::Result<()> {
    let mut fail = 0;
    let license = root.license();

    for package in packages {
        if package.id == root.id {
            continue;
        }
        let can_include = license.can_include(&package.license());
        if let Some(can_include) = can_include {
            if !can_include {
                log::error!(
                    "{} cannot include package {}, license {} is incompatible with {}",
                    root.name,
                    package.name,
                    package.license(),
                    license
                );
                fail += 1;
            }
        } else {
            log::warn!("{} might not be able to include package {}, license {} is not known to be compatible with {}", root.name, package.name, package.license(), license);
        }
    }

    if fail > 0 {
        Err(anyhow!("Incompatible license"))
    } else {
        Ok(())
    }
}
