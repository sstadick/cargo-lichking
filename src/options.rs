use std::str::FromStr;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

// TODO
pub type PackageIdSpec = String;

/// Whether to list crates per license or licenses per crate.
#[derive(Copy, Clone, Debug)]
pub enum By {
    License,
    Crate,
}

/// [`SelectedPackage`] determines which packages to collection license information on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectedPackage {
    All,
    Default,
    Specific(PackageIdSpec),
}

/// [`Bundle`] controls how the license information is collected and displayed.
#[derive(Clone, Debug)]
pub enum Bundle {
    /// Write both the name and content of the license used by each dependency.
    Inline { file: Option<String> },
    /// Write only the name of the license use by each dependency.
    NameOnly { file: Option<String> },
    /// Output Rust code that can output the name and content of the license used by each dependency
    Source { file: Option<String> },
    /// Write the name of the license used by each dependency to a file, and write a file with the license content to dir.
    Split { file: Option<String>, dir: String },
}

#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Cmd {
    /// List licensing of all dependencies.
    List { by: By, package: SelectedPackage },
    /// Check that all dependencies have a compatible license with a package.
    Check { package: SelectedPackage },
    /// Bundle all dependencies licenses ready for distribution
    Bundle {
        variant: Bundle,
        package: SelectedPackage,
    },
    /// List dependencies of cargo-lichking
    ThirdParty { full: bool },
}

#[derive(Clone, Debug)]
pub struct Options {
    /// Use verbose output
    pub verbose: u32,
    /// Use quiet output
    pub quiet: bool,
    /// Use color in outputs
    pub color: Option<String>,
    /// Require that `Cargo.lock` and cache are up to date
    pub frozen: bool,
    /// Require `Cargo.lock` is up to date
    pub locked: bool,
    /// The [`Cmd`] to run
    pub cmd: Cmd,
}

impl By {
    fn args() -> Vec<Arg<'static, 'static>> {
        vec![Arg::with_name("by")
            .long("by")
            .takes_value(true)
            .possible_values(&["license", "crate"])
            .default_value("license")
            .help("Whether to list crates per license or licenses per crate")]
    }

    fn from_matches(matches: &ArgMatches) -> By {
        matches
            .value_of("by")
            .expect("defaulted")
            .parse()
            .expect("constrained")
    }
}

impl SelectedPackage {
    fn args() -> Vec<Arg<'static, 'static>> {
        vec![
            Arg::with_name("all")
                .long("all")
                .help("Apply to all packages in workspace"),
            Arg::with_name("package")
                .short("p")
                .long("package")
                .takes_value(true)
                .value_name("NAME")
                .help("Package to apply this command to"),
        ]
    }

    fn help() -> &'static str {
        "\
            If the --package argument is given, then NAME is a package name which \
            indicates which package this command should apply to. If it is not given, \
            then the current package is used.

\
            All packages in the workspace are used if the `--all` flag is supplied. \
            The `--all` flag may be supplied in the presence of a virtual manifest. \
        "
    }

    fn from_matches(matches: &ArgMatches) -> SelectedPackage {
        if matches.is_present("all") {
            SelectedPackage::All
        } else {
            matches
                .value_of("package")
                .map(|s| s.to_owned())
                .map_or(SelectedPackage::Default, SelectedPackage::Specific)
        }
    }
}

impl Bundle {
    fn args() -> Vec<Arg<'static, 'static>> {
        vec![
            Arg::with_name("variant")
                .long("variant")
                .takes_value(true)
                .possible_values(&["inline", "name-only", "source", "split"])
                .default_value("inline")
                .requires_if("split", "dir")
                .help("Use long help to see more.")
                .long_help(
                    "\
What sort of bundle to produce:

    inline:
        Output a single file to location specified by --file containing the
        name and content of the license used by each dependency

    name-only:
        Output a single file to location specified by --file containing just
        the name of the license used by each dependency

    source:
        Output a single file to location specified by --file containing Rust
        source with the name and content of the license used by each dependency

    split:
        Output a file to location specified by --file containing the name of
        the license used by each dependency, along with a folder at the location
        specified by --dir containing the text of each dependency's license in a
        separate file inside

\
                ",
                ),
            Arg::with_name("file")
                .long("file")
                .takes_value(true)
                .value_name("FILE")
                .help("The file to output to (standard out if not specified)"),
            Arg::with_name("dir")
                .long("dir")
                .takes_value(true)
                .value_name("DIR")
                .help("The directory to output to"),
        ]
    }

    fn from_matches(matches: &ArgMatches) -> Bundle {
        match matches.value_of("variant").expect("defaulted") {
            "inline" => Bundle::Inline {
                file: matches.value_of("file").map(ToOwned::to_owned),
            },
            "name-only" => Bundle::NameOnly {
                file: matches.value_of("file").map(ToOwned::to_owned),
            },
            "source" => Bundle::Source {
                file: matches.value_of("file").map(ToOwned::to_owned),
            },
            "split" => Bundle::Split {
                file: matches.value_of("file").map(ToOwned::to_owned),
                dir: matches.value_of("dir").expect("required").to_owned(),
            },
            variant => panic!("Unexpected variant value {}", variant),
        }
    }
}

impl Options {
    pub fn app(subcommand_required: bool) -> App<'static, 'static> {
        App::new("cargo")
            .bin_name("cargo")
            .subcommand(Options::subapp(subcommand_required))
            .setting(AppSettings::SubcommandRequiredElseHelp)
            .global_settings(&[
                AppSettings::ColorAuto,
                AppSettings::ColoredHelp,
                AppSettings::VersionlessSubcommands,
                AppSettings::DeriveDisplayOrder,
                AppSettings::UnifiedHelpMessage,
            ])
    }

    // For some reason setting SubcommandRequired on the "lichking" sub command
    // propogates down to its subcommands as well, need to work out what's
    // happening and open a clap ticket so this argument is not needed.
    //
    // For now, try parsing the args without the subcommand being required,
    // then if we don't get a subcommand re-parse with it required to get the
    // error output.
    pub fn subapp(subcommand_required: bool) -> App<'static, 'static> {
        let mut app = SubCommand::with_name("lichking")
            .author(clap::crate_authors!())
            .version(clap::crate_version!())
            .about(clap::crate_description!())
            .args(&Options::args())
            .subcommands(Options::subcommands());
        if subcommand_required {
            app = app.setting(AppSettings::SubcommandRequiredElseHelp);
        }
        app
    }

    pub fn args() -> Vec<Arg<'static, 'static>> {
        vec![
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Use verbose output (-vv very verbose output)"),
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .help("Use quiet output"),
            Arg::with_name("color")
                .long("color")
                .takes_value(true)
                .value_name("COLOR")
                .possible_values(&["auto", "always", "never"])
                .help("Coloring"),
            Arg::with_name("frozen")
                .long("frozen")
                .help("Require Cargo.lock and cache are up to date"),
            Arg::with_name("locked")
                .long("locked")
                .help("Require Cargo.lock is up to date"),
        ]
    }

    pub fn subcommands() -> Vec<App<'static, 'static>> {
        vec![
            SubCommand::with_name("check")
                .about("Check that all dependencies have a compatible license with a package")
                .args(&SelectedPackage::args())
                .after_help(SelectedPackage::help()),
            SubCommand::with_name("list")
                .about("List licensing of all dependencies")
                .args(&By::args())
                .args(&SelectedPackage::args())
                .after_help(SelectedPackage::help()),
            SubCommand::with_name("bundle")
                .about("Bundle all dependencies licenses ready for distribution")
                .args(&Bundle::args())
                .args(&SelectedPackage::args())
                .after_help(SelectedPackage::help()),
            SubCommand::with_name("thirdparty")
                .about("List dependencies of cargo-lichking")
                .args(&[Arg::with_name("full")
                    .long("full")
                    .help("Whether to list license content for each dependency")]),
        ]
    }

    pub fn from_matches(matches: &ArgMatches) -> Options {
        let matches = matches.subcommand_matches("lichking").expect("required");
        Options {
            verbose: matches.occurrences_of("verbose") as u32,
            quiet: matches.is_present("quiet"),
            color: matches.value_of("color").map(ToOwned::to_owned),
            frozen: matches.is_present("frozen"),
            locked: matches.is_present("locked"),
            cmd: match matches.subcommand() {
                ("check", Some(matches)) => Cmd::Check {
                    package: SelectedPackage::from_matches(matches),
                },
                ("list", Some(matches)) => Cmd::List {
                    by: By::from_matches(matches),
                    package: SelectedPackage::from_matches(matches),
                },
                ("bundle", Some(matches)) => Cmd::Bundle {
                    variant: Bundle::from_matches(matches),
                    package: SelectedPackage::from_matches(matches),
                },
                ("thirdparty", Some(matches)) => Cmd::ThirdParty {
                    full: matches.is_present("full"),
                },
                (subcommand, _) => {
                    Options::app(true).get_matches();
                    panic!("Unexpected subcommand {}", subcommand)
                }
            },
        }
    }
}

impl FromStr for By {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "license" => Ok(By::License),
            "crate" => Ok(By::Crate),
            s => Err(format!("Cannot parse By from '{}'", s)),
        }
    }
}
