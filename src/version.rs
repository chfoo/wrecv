use std::sync::OnceLock;

use semver::Version;

pub fn get_crate_version_mmp() -> (u64, u64, u64) {
    static VERSION: OnceLock<(u64, u64, u64)> = OnceLock::new();

    VERSION
        .get_or_init(|| {
            let crate_version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0");

            let version = Version::parse(crate_version).unwrap();

            (version.major, version.minor, version.patch)
        })
        .to_owned()
}
