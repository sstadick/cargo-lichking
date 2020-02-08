mod bundle;
mod check;
mod discovery;
mod license;
mod licensed;
mod list;
mod load;
mod options;
mod query;
mod thirdparty;

use cargo_metadata::MetadataCommand;

use crate::options::{Cmd, Options};

fn main() {
    fn inner() -> anyhow::Result<()> {
        let matches = Options::app(false).get_matches();
        let options = Options::from_matches(&matches);

        let mut logger = pretty_env_logger::formatted_builder();
        if let Some(color) = options.color {
            logger.parse_write_style(&color);
        }
        logger.init();

        log::warn!("IANAL: This is not legal advice and is not guaranteed to be correct.");

        let opt_map = [
            (options.verbose > 0, "--verbose"),
            (options.verbose > 1, "--verbose"),
            (options.verbose > 2, "--verbose"),
            (options.verbose > 3, "--verbose"),
            (options.quiet, "--quiet"),
            (options.frozen, "--frozen"),
            (options.locked, "--locked"),
        ];

        let other_options = opt_map
            .iter()
            .filter(|(enabled, _)| *enabled)
            .map(|(_, opt)| (*opt).to_owned())
            .collect::<Vec<_>>();

        let metadata = MetadataCommand::new().other_options(other_options).exec()?;

        match options.cmd {
            Cmd::Check { package } => {
                let mut error = Ok(());
                let roots = load::resolve_roots(&metadata, package)?;
                for root in roots {
                    let roots = [root];
                    let packages = load::resolve_packages(&metadata, &roots)?;
                    if let Err(err) = check::run(root, &packages) {
                        error = Err(err);
                    }
                }
                error?;
            }

            Cmd::List { by, package } => {
                let roots = load::resolve_roots(&metadata, package)?;
                let packages = load::resolve_packages(&metadata, &roots)?;
                list::run(&packages, by)?;
            }

            Cmd::Bundle { variant, package } => {
                let roots = load::resolve_roots(&metadata, package)?;
                let packages = load::resolve_packages(&metadata, &roots)?;
                bundle::run(&roots, &packages, variant)?;
            }

            Cmd::ThirdParty { full } => {
                println!(
                    "cargo-lichking uses some third party libraries under their own license terms:"
                );
                println!();
                for krate in thirdparty::CRATES {
                    print!(
                        " * {} v{} under the terms of {}",
                        krate.name, krate.version, krate.licenses.name
                    );
                    if full {
                        println!(":");
                        let mut first = true;
                        for license in krate.licenses.licenses {
                            if first {
                                first = false;
                            } else {
                                println!();
                                println!("    ===============");
                            }
                            println!();
                            if let Some(text) = license.text {
                                for line in text.lines() {
                                    println!("    {}", line);
                                }
                            } else {
                                println!("    Missing {} license text", license.name);
                            }
                        }
                    }
                    println!();
                }
            }
        }

        Ok(())
    }

    if let Err(error) = inner() {
        log::error!("{}", error);
        std::process::exit(1);
    }
}
