use cargo::core::Package;
use cargo::{CargoResult, Config};

use crate::licensed::Licensed;

pub fn run(root: &Package, packages: Vec<Package>, config: &Config) -> CargoResult<()> {
    let mut fail = 0;
    let license = root.license();

    for package in packages {
        if &package == root {
            continue;
        }
        let can_include = license.can_include(&package.license());
        if let Some(can_include) = can_include {
            if !can_include {
                config.shell().error(format!(
                    "{} cannot include package {}, license {} is incompatible with {}",
                    root.name(),
                    package.name(),
                    package.license(),
                    license
                ))?;
                fail += 1;
            }
        } else {
            config.shell().warn(format!("{} might not be able to include package {}, license {} is not known to be compatible with {}", root.name(), package.name(), package.license(), license))?;
        }
    }

    if fail > 0 {
        failure::bail!("Incompatible license")
    } else {
        Ok(())
    }
}
