use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use slug::slugify;

#[derive(Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
#[allow(non_camel_case_types)]
pub enum License {
    // Licenses specified in the [SPDX License List](https://spdx.org/licenses/)
    Unlicense,
    BSD_0_Clause,
    CC0_1_0,
    MIT,
    X11,
    BSD_2_Clause,
    BSD_3_Clause,
    Apache_2_0,
    LGPL_2_0,
    LGPL_2_1,
    LGPL_2_1Plus,
    LGPL_3_0,
    LGPL_3_0Plus,
    MPL_1_1,
    MPL_2_0,
    GPL_2_0,
    GPL_2_0Plus,
    GPL_3_0,
    GPL_3_0Plus,
    AGPL_3_0,
    AGPL_3_0Plus,

    // Special cases
    Custom(String),
    File(PathBuf),
    Multiple(Vec<License>),
    Unspecified,
}

impl Default for License {
    fn default() -> License {
        License::Unspecified
    }
}

macro_rules! compatibility {
  ($s:expr, $o:expr, { $($a:pat => [$($b:pat),+])+ }) => {
    match $s {
      $(
        $a => if let $($b)|+ = $o {
          return Some(true);
        }
      ),*
    }
  };
}

impl License {
    pub fn can_include(&self, other: &License) -> Option<bool> {
        use self::License::*;

        if let Unspecified = *other {
            return Some(false);
        }

        if let Custom(_) = *self {
            return None;
        }
        if let Custom(_) = *other {
            return None;
        }
        if let File(_) = *self {
            return None;
        }
        if let File(_) = *other {
            return None;
        }

        if let Multiple(ref licenses) = *self {
            for license in licenses {
                if let Some(can_include) = license.can_include(other) {
                    if !can_include {
                        return Some(false);
                    }
                } else {
                    return None;
                }
            }
            return Some(true);
        }

        if let Multiple(ref licenses) = *other {
            let mut seen_none = false;
            for license in licenses {
                if let Some(can_include) = self.can_include(license) {
                    if can_include {
                        return Some(true);
                    }
                } else {
                    seen_none = true;
                }
            }
            return if seen_none { None } else { Some(false) };
        }

        if let LGPL_2_0 = *self {
            return None; /* TODO: unknown */
        }
        if let LGPL_2_0 = *other {
            return None; /* TODO: unknown */
        }

        compatibility!(*self, *other, {
            Unspecified         => [Unlicense, MIT, X11, BSD_2_Clause, BSD_3_Clause]

            LGPL_2_0     => [LGPL_2_0] // TODO: probably allows more

            Unlicense    => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11]
            BSD_0_Clause => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11]
            CC0_1_0      => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11]
            MIT          => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11]
            X11          => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11]
            BSD_2_Clause => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause]
            BSD_3_Clause => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause]
            Apache_2_0   => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, Apache_2_0]
            MPL_1_1      => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_1_1]
            MPL_2_0      => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, Apache_2_0, MPL_2_0]
            LGPL_2_1Plus => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, LGPL_2_1Plus]
            LGPL_2_1     => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, LGPL_2_1Plus, LGPL_2_1]
            LGPL_3_0Plus => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, Apache_2_0, LGPL_2_1Plus, LGPL_3_0Plus]
            LGPL_3_0     => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, Apache_2_0, LGPL_2_1Plus, LGPL_3_0Plus, LGPL_3_0]
            GPL_2_0Plus  => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, LGPL_2_1Plus, LGPL_2_1, GPL_2_0Plus]
            GPL_2_0      => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, LGPL_2_1Plus, LGPL_2_1, GPL_2_0Plus, GPL_2_0]
            GPL_3_0Plus  => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, Apache_2_0, LGPL_2_1Plus, LGPL_2_1, GPL_2_0Plus, GPL_3_0Plus]
            GPL_3_0      => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, Apache_2_0, LGPL_2_1Plus, LGPL_2_1, GPL_2_0Plus, GPL_3_0Plus, GPL_3_0]
            AGPL_3_0Plus => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, Apache_2_0, LGPL_2_1Plus, LGPL_2_1, GPL_2_0Plus, GPL_3_0Plus, GPL_3_0, AGPL_3_0Plus]
            AGPL_3_0     => [Unlicense, BSD_0_Clause, CC0_1_0, MIT, X11, BSD_2_Clause, BSD_3_Clause, MPL_2_0, Apache_2_0, LGPL_2_1Plus, LGPL_2_1, GPL_2_0Plus, GPL_3_0Plus, GPL_3_0, AGPL_3_0Plus, AGPL_3_0]

            // TODO: These are `unreachable!()`, can't figure out a nice way to allow this in the macro...
            Custom(_)    => [MIT]
            File(_)      => [MIT]
            Multiple(_)  => [MIT]
        });

        Some(false)
    }

    pub fn template(&self) -> Option<&'static str> {
        Some(match *self {
            License::Unlicense => include_str!("licenses/Unlicense"),
            License::MIT => include_str!("licenses/MIT"),
            License::Apache_2_0 => include_str!("licenses/Apache-2.0"),
            License::BSD_3_Clause => include_str!("licenses/BSD-3-Clause"),
            License::Multiple(_) => panic!("TODO: Refactor multiple handling"),
            _ => return None,
        })
    }
}

impl FromStr for License {
    type Err = core::convert::Infallible;

    fn from_str(s: &str) -> Result<License, core::convert::Infallible> {
        Ok(match s.trim() {
            "Unlicense" => License::Unlicense,
            "0BSD" => License::BSD_0_Clause,
            "CC0-1.0" => License::CC0_1_0,
            "MIT" => License::MIT,
            "X11" => License::X11,
            "BSD-2-Clause" => License::BSD_2_Clause,
            "BSD-3-Clause" => License::BSD_3_Clause,
            "Apache-2.0" => License::Apache_2_0,
            "LGPL-2.0-only" | "LGPL-2.0" => License::LGPL_2_0,
            "LGPL-2.1-only" | "LGPL-2.1" => License::LGPL_2_1,
            "LGPL-2.1-or-later" | "LGPL-2.1+" => License::LGPL_2_1Plus,
            "LGPL-3.0-only" | "LGPL-3.0" => License::LGPL_3_0,
            "LGPL-3.0-or-later" | "LGPL-3.0+" => License::LGPL_3_0Plus,
            "MPL-1.1" => License::MPL_1_1,
            "MPL-2.0" => License::MPL_2_0,
            "GPL-2.0-only" | "GPL-2.0" => License::GPL_2_0,
            "GPL-2.0-or-later" | "GPL-2.0+" => License::GPL_2_0Plus,
            "GPL-3.0-only" | "GPL-3.0" => License::GPL_3_0,
            "GPL-3.0-or-later" | "GPL-3.0+" => License::GPL_3_0Plus,
            "AGPL-3.0-only" | "AGPL-3.0" => License::AGPL_3_0,
            "AGPL-3.0-or-later" | "AGPL-3.0+" => License::AGPL_3_0Plus,
            s if s.contains('/') || s.contains(" OR ") => {
                let mut licenses = s
                    .split('/')
                    .flat_map(|s| s.split(" OR "))
                    .map(str::parse)
                    .map(Result::unwrap)
                    .collect::<Vec<License>>();
                licenses.sort();
                License::Multiple(licenses)
            }
            s => License::Custom(s.to_owned()),
        })
    }
}

impl fmt::Display for License {
    fn fmt(&self, w: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            License::Unlicense => write!(w, "Unlicense"),
            License::BSD_0_Clause => write!(w, "0BSD"),
            License::CC0_1_0 => write!(w, "CC0-1.0"),
            License::MIT => write!(w, "MIT"),
            License::X11 => write!(w, "X11"),
            License::BSD_2_Clause => write!(w, "BSD-2-Clause"),
            License::BSD_3_Clause => write!(w, "BSD-3-Clause"),
            License::Apache_2_0 => write!(w, "Apache-2.0"),
            License::LGPL_2_0 => write!(w, "LGPL-2.0-only"),
            License::LGPL_2_1 => write!(w, "LGPL-2.1-only"),
            License::LGPL_2_1Plus => write!(w, "LGPL-2.1-or-later"),
            License::LGPL_3_0 => write!(w, "LGPL-3.0-only"),
            License::LGPL_3_0Plus => write!(w, "LGPL-3.0-or-later"),
            License::MPL_1_1 => write!(w, "MPL-1.1"),
            License::MPL_2_0 => write!(w, "MPL-2.0"),
            License::GPL_2_0 => write!(w, "GPL-2.0-only"),
            License::GPL_2_0Plus => write!(w, "GPL-2.0-or-later"),
            License::GPL_3_0 => write!(w, "GPL-3.0-only"),
            License::GPL_3_0Plus => write!(w, "GPL-3.0-or-later"),
            License::AGPL_3_0 => write!(w, "AGPL-3.0-only"),
            License::AGPL_3_0Plus => write!(w, "AGPL-3.0-or-later"),
            License::Custom(ref s) => write!(w, "{}", s),
            License::File(ref f) => {
                write!(w, "License specified in file ({})", f.to_string_lossy())
            }
            License::Multiple(ref ls) => {
                write!(w, "{}", ls[0])?;
                for l in ls.iter().skip(1) {
                    write!(w, " / {}", l)?;
                }
                Ok(())
            }
            License::Unspecified => write!(w, "No license specified"),
        }
    }
}

impl License {
    /// slugified synonyms
    pub fn synonyms(&self) -> Vec<String> {
        match self {
            License::Apache_2_0 => vec![
                slugify(self.to_string()).to_lowercase(),
                String::from("apache"),
            ],
            License::Unlicense
            | License::BSD_0_Clause
            | License::CC0_1_0
            | License::MIT
            | License::X11
            | License::BSD_2_Clause
            | License::BSD_3_Clause
            | License::LGPL_2_0
            | License::LGPL_2_1
            | License::LGPL_2_1Plus
            | License::LGPL_3_0
            | License::LGPL_3_0Plus
            | License::MPL_1_1
            | License::MPL_2_0
            | License::GPL_2_0
            | License::GPL_2_0Plus
            | License::GPL_3_0
            | License::GPL_3_0Plus
            | License::AGPL_3_0
            | License::AGPL_3_0Plus
            | License::Custom(_)
            | License::File(_)
            | License::Multiple(_)
            | License::Unspecified => vec![slugify(self.to_string()).to_lowercase()],
        }
    }
}
