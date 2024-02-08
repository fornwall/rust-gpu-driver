use clap::ArgAction;

#[derive(Debug)]
pub struct Args {
    pub script: Option<String>,
    pub script_args: Vec<String>,
    pub base_path: Option<String>,
    pub pkg_path: Option<String>,
    pub gen_pkg_only: bool,
    pub cargo_output: bool,
    pub clear_cache: bool,
    pub debug: bool,
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
                .num_args(1..)
                .trailing_var_arg(true)
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
                .requires("shader")
                .conflicts_with_all(["debug"])
            )
            .arg(Arg::new("pkg_path")
                .help("Specify where to place the generated Cargo package")
                .long("pkg-path")
                .num_args(1)
                .requires("shader")
                .conflicts_with_all(["clear-cache"])
            );

        let mut m = app.get_matches();

        let script_and_args: Option<Vec<String>> = m
            .remove_many::<String>("shader")
            .map(|values| values.collect());
        let script;
        let script_args: Vec<String>;
        if let Some(script_and_args) = script_and_args {
            script = script_and_args.first().map(|s| s.to_string());
            script_args = if script_and_args.len() > 1 {
                Vec::from_iter(script_and_args[1..].iter().map(|s| s.to_string()))
            } else {
                Vec::new()
            };
        } else {
            script = None;
            script_args = Vec::new();
        }

        Self {
            script,
            script_args,

            base_path: m.get_one::<String>("base-path").map(Into::into),
            pkg_path: m.get_one::<String>("pkg_path").map(Into::into),
            gen_pkg_only: m.get_flag("gen_pkg_only"),
            cargo_output: m.get_flag("cargo-output"),
            clear_cache: m.get_flag("clear-cache"),
            debug: m.get_flag("debug"),
        }
    }
}
