use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use anyhow::anyhow;
use cargo_metadata::Package;

use crate::discovery::{
    better_find, find_generic_license_text, find_license_text, Confidence, LicenseText,
};
use crate::license::License;
use crate::licensed::Licensed;
use crate::options::Bundle;

#[derive(Debug)]
enum LicenseIssues {
    /// The LICENSE type files contents do not match the expected confidence
    LowConfidence {
        package_name: String,
        license: String,
        confidence: Confidence,
    },
    MultiplePossible {
        package_name: String,
        license: String,
        // TODO: list them?
    },
    /// There is no LICENSE type file for the given license
    Missing {
        package_name: String,
        license: String,
    },
    /// The package does not specify a LICENSE
    Unspecified { package_name: String },
    /// There is nothing to compare the content of the LICENSE type file to
    UnverifiableContent {
        package_name: String,
        license: String,
    },
}

/// Hold state that will be used to determine the overall exit value of the program
struct Context<'a> {
    roots_name: String,
    packages: &'a [&'a Package],
    issues: Vec<LicenseIssues>,

    missing_license: bool,
    low_quality_license: bool,
}

/// Collect all licenses for selected packages and display them as per [`Bundle`].
pub fn run(roots: &[&Package], packages: &[&Package], variant: Bundle) -> anyhow::Result<()> {
    let packages = {
        let mut packages = packages.to_owned();
        packages.sort_by_key(|p| (&p.name, &p.version));
        packages
    };

    let roots_name = {
        if roots.len() == 1 {
            format!("{} package", roots[0].name)
        } else {
            let mut roots_name = String::new();
            roots_name += roots[0].name.as_str();
            for root in roots.iter().take(roots.len() - 1).skip(1) {
                roots_name += ", ";
                roots_name += root.name.as_str();
            }
            roots_name += " and ";
            roots_name += roots.last().unwrap().name.as_str();
            roots_name += " packages";
            roots_name
        }
    };

    let mut context = Context {
        roots_name,
        packages: &packages,
        issues: vec![],
        missing_license: false,
        low_quality_license: false,
    };

    match variant {
        Bundle::Inline { file } => {
            if let Some(file) = file {
                inline(&mut context, &mut File::create(file)?)?;
            } else {
                inline(&mut context, &mut io::stdout())?;
            }
        }
        Bundle::NameOnly { file } => {
            if let Some(file) = file {
                name_only(&mut context, &mut File::create(file)?)?;
            } else {
                name_only(&mut context, &mut io::stdout())?;
            }
        }
        Bundle::Source { file } => {
            if let Some(file) = file {
                source(&mut context, &mut File::create(file)?)?;
            } else {
                source(&mut context, &mut io::stdout())?;
            }
        }
        Bundle::Split { file, dir } => {
            if let Some(file) = file {
                split(&mut context, &mut File::create(file)?, dir)?;
            } else {
                split(&mut context, &mut io::stdout(), dir)?;
            }
        }
    }

    if context.missing_license {
        log::error!(
            "
  Our liches failed to recognize a license in one or more packages.

  We would be very grateful if you could check the corresponding package
  directories (see the package specific message above) to see if there is an
  easily recognizable license file available.

  If there is please submit details to
      https://github.com/Nemo157/cargo-lichking/issues
  so we can make sure this license is recognized in the future.

  If there isn't you could submit an issue to the package's project asking
  them to include the text of their license in the built packages.",
        );
    }

    if context.low_quality_license {
        log::warn!(
            "\
             Our liches are very unsure about one or more licenses that were put into the \
             bundle. Please check the specific error messages above.",
        );
    }

    for issue in context.issues {
        // TODO: impl tostring
        // TODO: lower the log level
        log::error!("{:?}", issue);
    }

    if context.missing_license || context.low_quality_license {
        Err(anyhow!("Generating bundle finished with error(s)"))
    } else {
        Ok(())
    }
}

fn inline(context: &mut Context, out: &mut dyn io::Write) -> anyhow::Result<()> {
    writeln!(
        out,
        "The {} uses some third party libraries under their own license terms:",
        context.roots_name
    )?;
    writeln!(out)?;
    for package in context.packages {
        writeln!(
            out,
            " * {} {} under the terms of {}:",
            package.name,
            package.version,
            package.license(),
        )?;
        writeln!(out)?;
        inline_package(context, package, out)?;
        writeln!(out)?;
    }
    Ok(())
}

fn name_only(context: &mut Context, out: &mut dyn io::Write) -> anyhow::Result<()> {
    writeln!(
        out,
        "The {} uses some third party libraries under their own license terms:",
        context.roots_name
    )?;
    writeln!(out)?;
    for package in context.packages {
        writeln!(
            out,
            " * {} {} under the terms of {}",
            package.name,
            package.version,
            package.license(),
        )?;
    }
    Ok(())
}

fn source(context: &mut Context, out: &mut dyn io::Write) -> anyhow::Result<()> {
    out.write_all(
        b"\
//! Licenses of dependencies
//!
//! This file was generated by [`cargo-lichking`](https://github.com/Nemo157/cargo-lichking)

pub struct License {
    pub name: &'static str,
    pub text: Option<&'static str>,
}

pub struct Licenses {
    pub name: &'static str,
    pub licenses: &'static [License],
}

pub struct LicensedCrate {
    pub name: &'static str,
    pub version: &'static str,
    pub licenses: Licenses,
}

pub const CRATES: &[LicensedCrate] = &[
",
    )?;
    for package in context.packages {
        source_package(context, package, out)?;
    }
    out.write_all(b"];\n")?;
    Ok(())
}

fn split<P: AsRef<Path>>(
    context: &mut Context,
    out: &mut dyn io::Write,
    dir: P,
) -> anyhow::Result<()> {
    fs::create_dir_all(dir.as_ref())?;
    writeln!(
        out,
        "The {} uses some third party libraries under their own license terms:",
        context.roots_name
    )?;
    writeln!(out)?;
    for package in context.packages {
        writeln!(
            out,
            " * {} {} under the terms of {}",
            package.name,
            package.version,
            package.license(),
        )?;
        split_package(context, package, dir.as_ref())?;
    }
    Ok(())
}

fn inline_package(
    context: &mut Context,
    package: &Package,
    out: &mut dyn io::Write,
) -> anyhow::Result<()> {
    let license = package.license();

    match license {
        License::Unspecified => {
            log::error!("{} does not specify a license", package.name);
            context.issues.push(LicenseIssues::Unspecified {
                package_name: package.name.clone(),
            });
        }
        License::Multiple(licenses) => {
            let mut first = true;
            for license in licenses {
                if first {
                    first = false;
                } else {
                    writeln!(out)?;
                    writeln!(out, "    ===============")?;
                    writeln!(out)?;
                }
                inline_license(context, package, &license, out)?;
            }
        }
        license => {
            inline_license(context, package, &license, out)?;
        }
    }
    writeln!(out)?;
    Ok(())
}

fn source_package(
    context: &mut Context,
    package: &Package,
    out: &mut dyn io::Write,
) -> anyhow::Result<()> {
    eprintln!("Source_package");
    let license = package.license();
    let license_name = license.to_string();
    match license {
        License::Unspecified => {
            log::error!("{} does not specify a license", package.name);
            context.issues.push(LicenseIssues::Unspecified {
                package_name: package.name.clone(),
            });
        }
        License::Multiple(licenses) => {
            writeln!(
                out,
                "
    LicensedCrate {{
        name: {:?},
        version: {:?},
        licenses: Licenses {{
            name: {:?},
            licenses: &[",
                package.name,
                package.version.to_string(),
                license_name
            )?;
            for license in licenses {
                let texts = better_find(package, &license)?;
                let text = (choose(context, package, &license, texts)?)
                    .map_or_else(|| "None".to_owned(), |t| format!("Some({:?})", t.text));
                writeln!(
                    out,
                    "
                License {{
                    name: {:?},
                    text: {},
                }},",
                    license.to_string(),
                    text
                )?;
            }
            writeln!(
                out,
                "
            ],
        }},
    }},"
            )?;
        }
        license => {
            let texts = better_find(package, &license)?;
            let text = (choose(context, package, &license, texts)?)
                .map_or_else(|| "None".to_owned(), |t| format!("Some({:?})", t.text));
            writeln!(
                out,
                "
    LicensedCrate {{
        name: {:?},
        version: {:?},
        licenses: Licenses {{
            name: {:?},
            licenses: &[
                License {{
                    name: {:?},
                    text: {},
                }},
            ],
        }},
    }},",
                package.name,
                package.version.to_string(),
                license.to_string(),
                license.to_string(),
                text
            )?;
        }
    }
    writeln!(out)?;
    Ok(())
}

fn split_package(context: &mut Context, package: &Package, dir: &Path) -> anyhow::Result<()> {
    let license = package.license();
    let mut file = File::create(dir.join(package.name.as_str()))?;
    match license {
        License::Unspecified => {
            log::error!("{} does not specify a license", package.name);
            context.issues.push(LicenseIssues::Unspecified {
                package_name: package.name.clone(),
            });
        }
        License::Multiple(licenses) => {
            let mut first = true;
            for license in licenses {
                if first {
                    first = false;
                } else {
                    writeln!(file)?;
                    writeln!(file, "===============")?;
                    writeln!(file)?;
                }
                let texts = better_find(package, &license)?;
                if let Some(text) = choose(context, package, &license, texts)? {
                    file.write_all(text.text.as_bytes())?;
                }
            }
        }
        license => {
            let texts = better_find(package, &license)?;
            if let Some(text) = choose(context, package, &license, texts)? {
                file.write_all(text.text.as_bytes())?;
            }
        }
    }
    Ok(())
}

fn inline_license(
    context: &mut Context,
    package: &Package,
    license: &License,
    out: &mut dyn io::Write,
) -> anyhow::Result<()> {
    let texts = better_find(package, license)?;
    if let Some(text) = choose(context, package, license, texts)? {
        for line in text.text.lines() {
            writeln!(out, "    {}", line)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn choose(
    context: &mut Context,
    package: &Package,
    license: &License,
    texts: Vec<LicenseText>,
) -> anyhow::Result<Option<LicenseText>> {
    let (mut confident, texts): (Vec<LicenseText>, Vec<LicenseText>) = texts
        .into_iter()
        .partition(|text| text.confidence == Confidence::Confident);
    let (mut semi_confident, mut unconfident): (Vec<LicenseText>, Vec<LicenseText>) = texts
        .into_iter()
        .partition(|text| text.confidence == Confidence::SemiConfident);
    let (mut unsure, mut no_template): (Vec<LicenseText>, Vec<LicenseText>) = unconfident
        .into_iter()
        .partition(|text| text.confidence == Confidence::Unsure);

    Ok(Some({
        if confident.len() == 1 {
            confident.swap_remove(0)
        } else if confident.len() > 1 {
            log::error!(
                "{} has multiple candidates for license {}:",
                package.name,
                license
            );
            for text in &confident {
                log::error!("    {}", text.path.display());
                context.issues.push(LicenseIssues::MultiplePossible {
                    package_name: package.name.clone(),
                    license: license.to_string(),
                    // TODO: add paths and confidence
                });
            }
            confident.swap_remove(0)
        } else if semi_confident.len() == 1 {
            log::warn!(
                "{} has only a low-confidence candidate for license {}:\n    {}",
                package.name,
                license,
                semi_confident[0].path.display(),
            );
            context.issues.push(LicenseIssues::LowConfidence {
                package_name: package.name.clone(),
                license: license.to_string(),
                confidence: semi_confident[0].confidence,
            });
            semi_confident.swap_remove(0)
        } else if semi_confident.len() > 1 {
            eprintln!("Setting low quality (semi-confident) for {:?}", license);
            context.low_quality_license = true;
            log::error!(
                "{} has multiple low-confidence candidates for license {}:",
                package.name,
                license
            );
            for text in &semi_confident {
                log::error!("    {}", text.path.display());
                context.issues.push(LicenseIssues::MultiplePossible {
                    package_name: package.name.clone(),
                    license: license.to_string(),
                    // TODO: add paths and confidence
                });
            }
            semi_confident.swap_remove(0)
        } else if unsure.len() == 1 {
            eprintln!("Setting low quality (unconfident) for {:?}", license);
            context.low_quality_license = true;
            log::warn!(
                "{} has only a very low-confidence candidate for license {}:\n    {}",
                package.name,
                license,
                unsure[0].path.display(),
            );
            context.issues.push(LicenseIssues::LowConfidence {
                package_name: package.name.clone(),
                license: license.to_string(),
                confidence: unsure[0].confidence,
            });
            unsure.swap_remove(0)
        } else if unsure.len() > 1 {
            context.low_quality_license = true;
            log::error!(
                "{} has multiple very low-confidence candidates for license {}:",
                package.name,
                license
            );
            for text in &unsure {
                log::error!("    {}", text.path.display());
                context.issues.push(LicenseIssues::MultiplePossible {
                    package_name: package.name.clone(),
                    license: license.to_string(),
                    // TODO: add paths and confidence
                });
            }
            unsure.swap_remove(0)
        } else if no_template.len() == 1 {
            eprintln!("Setting no template (unconfident) for {:?}", license);
            context.low_quality_license = true;
            log::warn!(
                "{} has only a no template low-confidence candidate for license {}:\n    {}",
                package.name,
                license,
                no_template[0].path.display(),
            );
            context.issues.push(LicenseIssues::UnverifiableContent {
                package_name: package.name.clone(),
                license: license.to_string(),
            });
            no_template.swap_remove(0)
        } else if no_template.len() > 1 {
            context.low_quality_license = true;
            log::error!(
                "{} has multiple no template low-confidence candidates for license {}:",
                package.name,
                license
            );
            for text in &no_template {
                log::error!("    {}", text.path.display());
                context.issues.push(LicenseIssues::MultiplePossible {
                    package_name: package.name.clone(),
                    license: license.to_string(),
                    // TODO: add paths and confidence
                });
            }
            no_template.swap_remove(0)
        } else {
            log::error!(
                "{} has no candidate texts for license {} in {}",
                package.name,
                license,
                package.manifest_path.parent().unwrap().display()
            );
            context.missing_license = true;
            context.issues.push(LicenseIssues::Missing {
                package_name: package.name.clone(),
                license: license.to_string(),
            });
            return Ok(None);
        }
    }))
}
