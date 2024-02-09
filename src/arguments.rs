use clap::{builder::PossibleValue, ArgAction};

#[derive(Debug)]
pub struct Args {
    pub base_path: Option<String>,
    pub cargo_output: bool,
    pub output_path: Option<String>,
    pub clear_cache: bool,
    pub debug: bool,
    pub gen_pkg_only: bool,
    pub pkg_path: Option<String>,
    pub script: Option<String>,
    pub target: String,
}

impl Args {
    pub fn parse() -> Self {
        use clap::{Arg, Command};
        let version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown");
        let about = r#"Compile rust-gpu shader files to spir-v"#;

        let app = Command::new(crate::consts::PROGRAM_NAME)
            .bin_name(crate::consts::PROGRAM_NAME)
            .version(version)
            .about(about)
            .arg(Arg::new("shader")
                .index(1)
                .help("Shader source file to compile")
                .required_unless_present_any(
                    ["clear-cache"].iter()
                )
                .num_args(1)
            )
            .arg(Arg::new("base-path")
                .help("Base path for resolving dependencies")
                .short('b')
                .long("base-path")
                .num_args(1)
            )
            .arg(Arg::new("cargo-output")
                .help("Show output from cargo when building")
                .short('c')
                .long("cargo-output")
                .action(ArgAction::SetTrue)
                .requires("shader")
            )
            .arg(Arg::new("debug")
                .help("Build a debug executable, not an optimised one")
                .long("debug")
                .action(ArgAction::SetTrue)
            )
            .arg(Arg::new("clear-cache")
                .help("Clears out the script cache")
                .long("clear-cache")
                .action(ArgAction::SetTrue),
            )
            .arg(Arg::new("gen_pkg_only")
                .help("Generate the Cargo package and print the path to it, but don't compile or run it")
                .long("package")
                .short('p')
                .action(ArgAction::SetTrue)
                .conflicts_with_all(["debug"])
            )
            .arg(Arg::new("output")
                .help("Write output to <output>. A file name of '-' represents standard output.")
                .long("output")
                .short('o')
                .num_args(1)
                .conflicts_with_all(["gen_pkg_only"])
            )
            .arg(Arg::new("target")
                .help("SPIR-V target")
                .long("target")
                .short('t')
                .num_args(1)
                // XXX: https://embarkstudios.github.io/rust-gpu/book/platform-support.html
                .value_parser([
                    PossibleValue::new("spirv-unknown-spv1.0"),
                    PossibleValue::new("spirv-unknown-spv1.1"),
                    PossibleValue::new("spirv-unknown-spv1.2"),
                    PossibleValue::new("spirv-unknown-spv1.3"),
                    PossibleValue::new("spirv-unknown-spv1.4"),
                    PossibleValue::new("spirv-unknown-spv1.5"),
                    PossibleValue::new("spirv-unknown-vulkan1.0"),
                    PossibleValue::new("spirv-unknown-vulkan1.1"),
                    PossibleValue::new("spirv-unknown-vulkan1.1spv1.4"),
                    PossibleValue::new("spirv-unknown-vulkan1.2"),
                    PossibleValue::new("spirv-unknown-webgpu0"),
                    PossibleValue::new("spirv-unknown-opengl4.0"),
                    PossibleValue::new("spirv-unknown-opengl4.1"),
                    PossibleValue::new("spirv-unknown-opengl4.2"),
                    PossibleValue::new("spirv-unknown-opengl4.3"),
                    PossibleValue::new("spirv-unknown-opengl4.4"),
                    PossibleValue::new("spirv-unknown-opengl4.5"),
                ])
                .default_value("spirv-unknown-vulkan1.1")
                .conflicts_with_all(["gen_pkg_only"])
            )
            .arg(Arg::new("pkg_path")
                .help("Specify where to place the generated Cargo package")
                .long("pkg-path")
                .num_args(1)
                .requires("shader")
                .conflicts_with_all(["clear-cache"])
            );

        let m = app.get_matches();

        Self {
            script: m.get_one::<String>("shader").map(Into::into),
            base_path: m.get_one::<String>("base-path").map(Into::into),
            pkg_path: m.get_one::<String>("pkg_path").map(Into::into),
            gen_pkg_only: m.get_flag("gen_pkg_only"),
            cargo_output: m.get_flag("cargo-output"),
            output_path: m.get_one::<String>("output").map(Into::into),
            clear_cache: m.get_flag("clear-cache"),
            debug: m.get_flag("debug"),
            target: m.get_one::<String>("target").map(Into::into).unwrap(),
        }
    }
}
