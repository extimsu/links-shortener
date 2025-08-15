
            /// Returns the `rustc` SemVer version and additional metadata
            /// like the git short hash and build date.
            pub fn version_meta() -> VersionMeta {
                VersionMeta {
                    semver: Version {
                        major: 1,
                        minor: 89,
                        patch: 0,
                        pre: vec![],
                        build: vec![],
                    },
                    host: "aarch64-apple-darwin".to_owned(),
                    short_version_string: "rustc 1.89.0 (29483883e 2025-08-04)".to_owned(),
                    commit_hash: Some("29483883eed69d5fb4db01964cdf2af4d86e9cb2".to_owned()),
                    commit_date: Some("2025-08-04".to_owned()),
                    build_date: None,
                    channel: Channel::Stable,
                }
            }
            