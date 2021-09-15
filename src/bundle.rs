use std::env::var;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Component, Path};

use anyhow::anyhow;
use cargo_metadata::Package;

use crate::discovery::{better_find, Confidence, LicenseText};
use crate::license::{self, License};
use crate::licensed::Licensed;
use crate::options::Bundle;

/// A Lich represents a found set license texts for a given package - license combo
struct Lich {
    package: Package,
    license: License,
    texts: FoundTexts,
}

impl Lich {
    /// Build-a-lich workshop
    fn to_lich(package: &Package, license: License, texts: FoundTexts) -> Lich {
        Self {
            package: package.clone(),
            license,
            texts,
        }
    }
}

#[derive(Debug)]
enum LicenseInfo {
    /// The LICENSE type files contents do not match the expected confidence
    MultiplePossibleLicenseFiles,
    /// There is no LICENSE type file for the given license
    MissingLicenseFile,
    Confident,

    SemiConfident,
    Unsure,
    /// There is nothing to compare the content of the LICENSE type file to
    NoTemplate,
    /// The package does not specify a LICENSE
    UnspecifiedLicenseInPackage,
}

/// Hold state that will be used to determine the overall exit value of the program
struct Context<'a> {
    roots_name: String,
    packages: &'a [&'a Package],
    issues: Vec<LicenseInfo>,
    liches: Vec<Lich>,

    missing_license: bool,
    low_quality_license: bool,
}

fn inline_writer(
    maybe_file: Option<&String>,
    liches: Vec<Lich>,
    roots_name: String,
) -> anyhow::Result<()> {
    let mut writer: Box<dyn Write> = if let Some(file) = maybe_file {
        Box::new(BufWriter::new(File::create(file)?))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };

    writeln!(
        writer,
        "The {} uses some third party libraries under their own license terms:",
        roots_name
    )?;
    writeln!(writer)?;

    for lich in liches {
        writeln!(
            writer,
            " * {} {} under the terms of {}:",
            lich.package.name,
            lich.package.version,
            lich.package.license()
        )?;
        writeln!(writer)?;
        match lich.license {
            License::Unspecified => unimplemented!(),
            License::Multiple(licenses) => {
                let FoundTexts::Multiple(texts) = lich.texts;
                for (i, license) in licenses.iter().enumerate() {
                    let (best_choice, info) = texts[i];
                    // TODO: do some logging and filtering here?
                    match best_choice {
                        BestChoice::Single(text) => {
                            for line in text.text.lines() {
                                writeln!(writer, "    {}", line)?;
                            }
                        }
                        BestChoice::Multiple(texts) => {
                            for line in texts[0].text.lines() {
                                writeln!(writer, "    {}", line)?;
                            }
                        }
                        BestChoice::None => {
                            writeln!(writer, "    :(")?;
                        }
                    }
                }
            }
            license => {
                let FoundTexts::Single(best_choice, info) = lich.texts;
            }
        }
        writeln!(writer)?;
    }

    Ok(())
}

// Seem slike the thing to do is to make Lich have to make less choices in serialization
// print warnings / errors as they are seen and apply filters on the go
// The accumlated Vec<Lich> should be pretty much complete

impl Bundle {
    fn write_output(&self, liches: Vec<Lich>) -> anyhow::Result<()> {
        match self {
            Bundle::Inline { file } => inline_writer(file.as_ref(), liches),
            _ => unimplemented!(),
        }
    }
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
        liches: vec![],
        missing_license: false,
        low_quality_license: false,
    };

    let liches: Vec<_> = packages.iter().map(|&p| get_lich(p)).collect();
    match variant {
        Bundle::Inline { file } => {
            if let Some(file) = file {
            } else {
            }
        }
        _ => unimplemented!(),
    }

    // match variant {
    //     Bundle::Inline { file } => {
    //         if let Some(file) = file {
    //             inline(&mut context, &mut File::create(file)?)?;
    //         } else {
    //             inline(&mut context, &mut io::stdout())?;
    //         }
    //     }
    //     Bundle::NameOnly { file } => {
    //         unimplemented!()
    //         // if let Some(file) = file {
    //         //     name_only(&mut context, &mut File::create(file)?)?;
    //         // } else {
    //         //     name_only(&mut context, &mut io::stdout())?;
    //         // }
    //     }
    //     Bundle::Source { file } => {
    //         unimplemented!()
    //         // if let Some(file) = file {
    //         //     source(&mut context, &mut File::create(file)?)?;
    //         // } else {
    //         //     source(&mut context, &mut io::stdout())?;
    //         // }
    //     }
    //     Bundle::Split { file, dir } => {
    //         unimplemented!()
    //         // if let Some(file) = file {
    //         //     split(&mut context, &mut File::create(file)?, dir)?;
    //         // } else {
    //         //     split(&mut context, &mut io::stdout(), dir)?;
    //         // }
    //     }
    // }

    // TODO: standardized writing of liches here

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

// fn inline(context: &mut Context, out: &mut dyn io::Write) -> anyhow::Result<()> {
//     writeln!(
//         out,
//         "The {} uses some third party libraries under their own license terms:",
//         context.roots_name
//     )?;
//     writeln!(out)?;
//     for package in context.packages {
//         writeln!(
//             out,
//             " * {} {} under the terms of {}:",
//             package.name,
//             package.version,
//             package.license(),
//         )?;
//         writeln!(out)?;
//         inline_package(context, package, out)?;
//         writeln!(out)?;
//     }
//     Ok(())
// }

// fn inline_package(
//     context: &mut Context,
//     package: &Package,
//     out: &mut dyn io::Write,
// ) -> anyhow::Result<Lich> {
//     let mut liches = vec![];
//     let license = package.license();

//     match license {
//         License::Unspecified => Lich::to_lich(
//             package,
//             &license,
//             vec![],
//             vec![LicenseIssue::UnspecifiedLicenseInPackage {
//                 package_name: package.name.clone(),
//             }],
//         ),
//         License::Multiple(licenses) => {
//             let mut first = true;
//             for license in licenses {
//                 if first {
//                     first = false;
//                 } else {
//                     writeln!(out)?;
//                     writeln!(out, "    ===============")?;
//                     writeln!(out)?;
//                 }
//                 inline_license(context, package, &license, out)?;
//             }
//         }
//         license => {
//             inline_license(context, package, &license, out)?;
//         }
//     }
//     writeln!(out)?;
//     Ok(())
// }

// fn inline_license(
//     context: &mut Context,
//     package: &Package,
//     license: &License,
//     out: &mut dyn io::Write,
// ) -> anyhow::Result<()> {
//     let texts = better_find(package, license)?;
//     if let Some(text) = choose(context, package, license, texts)? {
//         for line in text.text.lines() {
//             writeln!(out, "    {}", line)?;
//         }
//     }
//     Ok(())
// }

/// Get the licenses for a given package and their corresponding text
fn get_lich(package: &Package) -> anyhow::Result<Lich> {
    let license = package.license();

    let results = match &license {
        License::Unspecified => {
            FoundTexts::Single(BestChoice::None, LicenseInfo::UnspecifiedLicenseInPackage)
        }
        License::Multiple(licenses) => {
            let mut choices = vec![];
            for license in licenses {
                let texts = better_find(package, license)?;
                choices.push(choose(package, license, texts));
            }
            FoundTexts::Multiple(choices)
        }
        license => {
            let texts = better_find(package, license)?;
            let (best, conf) = choose(package, &license, texts);
            FoundTexts::Single(best, conf)
        }
    };
    Ok(Lich::to_lich(package, license, results))
}

enum FoundTexts {
    Single(BestChoice, LicenseInfo),
    Multiple(Vec<(BestChoice, LicenseInfo)>),
}

enum BestChoice {
    Single(LicenseText),
    Multiple(Vec<LicenseText>),
    None,
}

struct Choice {
    best: BestChoice,
    confidence: Confidence,
}

/// Choose the highest confidence license of all possible licenses.
#[allow(clippy::too_many_lines)]
fn choose(
    package: &Package,
    license: &License,
    texts: Vec<LicenseText>,
) -> (BestChoice, LicenseInfo) {
    // Partition licnese texts by confidense
    let (mut confident, texts): (Vec<LicenseText>, Vec<LicenseText>) = texts
        .into_iter()
        .partition(|text| text.confidence == Confidence::Confident);
    let (mut semi_confident, unconfident): (Vec<LicenseText>, Vec<LicenseText>) = texts
        .into_iter()
        .partition(|text| text.confidence == Confidence::SemiConfident);
    let (mut unsure, mut no_template): (Vec<LicenseText>, Vec<LicenseText>) = unconfident
        .into_iter()
        .partition(|text| text.confidence == Confidence::Unsure);

    if confident.len() == 1 {
        (
            BestChoice::Single(confident.swap_remove(0)),
            LicenseInfo::Confident,
        )
    } else if confident.len() > 1 {
        (BestChoice::Multiple(confident), LicenseInfo::Confident)
    } else if semi_confident.len() == 1 {
        (
            BestChoice::Single(semi_confident.swap_remove(0)),
            LicenseInfo::SemiConfident,
        )
    } else if semi_confident.len() > 1 {
        (
            BestChoice::Multiple(semi_confident),
            LicenseInfo::SemiConfident,
        )
    } else if unsure.len() == 1 {
        (
            BestChoice::Single(unsure.swap_remove(0)),
            LicenseInfo::Unsure,
        )
    } else if unsure.len() > 1 {
        (BestChoice::Multiple(unsure), LicenseInfo::Unsure)
    } else if no_template.len() == 1 {
        (
            BestChoice::Single(no_template.swap_remove(0)),
            LicenseInfo::NoTemplate,
        )
    } else if no_template.len() > 1 {
        (BestChoice::Multiple(no_template), LicenseInfo::NoTemplate)
    } else {
        (BestChoice::None, LicenseInfo::MissingLicenseFile)
    }
}

// fn name_only(context: &mut Context, out: &mut dyn io::Write) -> anyhow::Result<()> {
//     writeln!(
//         out,
//         "The {} uses some third party libraries under their own license terms:",
//         context.roots_name
//     )?;
//     writeln!(out)?;
//     for package in context.packages {
//         writeln!(
//             out,
//             " * {} {} under the terms of {}",
//             package.name,
//             package.version,
//             package.license(),
//         )?;
//     }
//     Ok(())
// }

// fn source(context: &mut Context, out: &mut dyn io::Write) -> anyhow::Result<()> {
//     out.write_all(
//         b"\
// //! Licenses of dependencies
// //!
// //! This file was generated by [`cargo-lichking`](https://github.com/Nemo157/cargo-lichking)

// pub struct License {
//     pub name: &'static str,
//     pub text: Option<&'static str>,
// }

// pub struct Licenses {
//     pub name: &'static str,
//     pub licenses: &'static [License],
// }

// pub struct LicensedCrate {
//     pub name: &'static str,
//     pub version: &'static str,
//     pub licenses: Licenses,
// }

// pub const CRATES: &[LicensedCrate] = &[
// ",
//     )?;
//     for package in context.packages {
//         source_package(context, package, out)?;
//     }
//     out.write_all(b"];\n")?;
//     Ok(())
// }

// fn split<P: AsRef<Path>>(
//     context: &mut Context,
//     out: &mut dyn io::Write,
//     dir: P,
// ) -> anyhow::Result<()> {
//     fs::create_dir_all(dir.as_ref())?;
//     writeln!(
//         out,
//         "The {} uses some third party libraries under their own license terms:",
//         context.roots_name
//     )?;
//     writeln!(out)?;
//     for package in context.packages {
//         writeln!(
//             out,
//             " * {} {} under the terms of {}",
//             package.name,
//             package.version,
//             package.license(),
//         )?;
//         split_package(context, package, dir.as_ref())?;
//     }
//     Ok(())
// }

// fn source_package(
//     context: &mut Context,
//     package: &Package,
//     out: &mut dyn io::Write,
// ) -> anyhow::Result<()> {
//     let license = package.license();
//     let license_name = license.to_string();
//     match license {
//         License::Unspecified => {
//             context
//                 .issues
//                 .push(LicenseIssue::UnspecifiedLicenseInPackage {
//                     package_name: package.name.clone(),
//                 });
//         }
//         License::Multiple(licenses) => {
//             writeln!(
//                 out,
//                 "
//     LicensedCrate {{
//         name: {:?},
//         version: {:?},
//         licenses: Licenses {{
//             name: {:?},
//             licenses: &[",
//                 package.name,
//                 package.version.to_string(),
//                 license_name
//             )?;
//             for license in licenses {
//                 let texts = better_find(package, &license)?;
//                 let text = (choose(context, package, &license, texts)?)
//                     .map_or_else(|| "None".to_owned(), |t| format!("Some({:?})", t.text));
//                 writeln!(
//                     out,
//                     "
//                 License {{
//                     name: {:?},
//                     text: {},
//                 }},",
//                     license.to_string(),
//                     text
//                 )?;
//             }
//             writeln!(
//                 out,
//                 "
//             ],
//         }},
//     }},"
//             )?;
//         }
//         license => {
//             let texts = better_find(package, &license)?;
//             let text = (choose(context, package, &license, texts)?)
//                 .map_or_else(|| "None".to_owned(), |t| format!("Some({:?})", t.text));
//             writeln!(
//                 out,
//                 "
//     LicensedCrate {{
//         name: {:?},
//         version: {:?},
//         licenses: Licenses {{
//             name: {:?},
//             licenses: &[
//                 License {{
//                     name: {:?},
//                     text: {},
//                 }},
//             ],
//         }},
//     }},",
//                 package.name,
//                 package.version.to_string(),
//                 license.to_string(),
//                 license.to_string(),
//                 text
//             )?;
//         }
//     }
//     writeln!(out)?;
//     Ok(())
// }

// fn split_package(context: &mut Context, package: &Package, dir: &Path) -> anyhow::Result<()> {
//     let license = package.license();
//     let mut file = File::create(dir.join(package.name.as_str()))?;
//     match license {
//         License::Unspecified => {
//             context
//                 .issues
//                 .push(LicenseIssue::UnspecifiedLicenseInPackage {
//                     package_name: package.name.clone(),
//                 });
//         }
//         License::Multiple(licenses) => {
//             let mut first = true;
//             for license in licenses {
//                 if first {
//                     first = false;
//                 } else {
//                     writeln!(file)?;
//                     writeln!(file, "===============")?;
//                     writeln!(file)?;
//                 }
//                 let texts = better_find(package, &license)?;
//                 if let Some(text) = choose(context, package, &license, texts)? {
//                     file.write_all(text.text.as_bytes())?;
//                 }
//             }
//         }
//         license => {
//             let texts = better_find(package, &license)?;
//             if let Some(text) = choose(context, package, &license, texts)? {
//                 file.write_all(text.text.as_bytes())?;
//             }
//         }
//     }
//     Ok(())
// }
