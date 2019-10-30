use std::collections::HashMap;

use cargo::core::Package;
use cargo::CargoResult;
use itertools::Itertools;

use licensed::Licensed;
use options::By;

pub fn run(mut packages: Vec<Package>, by: By) -> CargoResult<()> {
    match by {
        By::License => {
            let mut license_to_packages = HashMap::new();

            for package in packages {
                license_to_packages
                    .entry(package.license())
                    .or_insert_with(Vec::new)
                    .push(package);
            }

            license_to_packages
                .iter()
                .sorted_by_key(|&(license, _)| license)
                .for_each(|(license, packages)| {
                    let packages = packages
                        .iter()
                        .map(|package| package.name())
                        .sorted()
                        .join(", ");
                    println!("{}: {}", license, packages);
                })
        }
        By::Crate => {
            packages.sort_by_key(|package| package.name().to_owned());
            for package in packages {
                println!("{}: {}", package.name(), package.license());
            }
        }
    }

    Ok(())
}
