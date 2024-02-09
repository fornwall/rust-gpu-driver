#![forbid(unsafe_code)]

mod arguments;
mod build;
mod consts;
mod defer;
mod error;
mod manifest;
mod platform;

use arguments::Args;
use log::{debug, error, info};
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::defer::Defer;
use crate::error::{MainError, MainResult};

use sha1::{Digest, Sha1};

fn main() {
    env_logger::init();

    match try_main() {
        Ok(code) => {
            std::process::exit(code);
        }
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
    }
}

fn try_main() -> MainResult<i32> {
    let args = arguments::Args::parse();
    info!("Arguments: {:?}", args);

    if args.clear_cache {
        clean_cache(0)?;
        if args.script.is_none() {
            println!("rust-gpu cache cleared.");
            return Ok(0);
        }
    }

    let input = {
        let script = args.script.clone().unwrap();
        let (path, mut file) =
            find_script(script.as_ref()).ok_or(format!("could not find script: {}", script))?;

        let script_name = path
            .file_stem()
            .map(|os| os.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".into());

        let mut body = String::new();
        file.read_to_string(&mut body)?;

        let script_path = std::env::current_dir()?.join(path);

        Input::File(script_name, script_path, body)
    };
    info!("input: {:?}", input);

    // Setup environment variables early so it's available at compilation time of scripts,
    // to allow e.g. include!(concat!(env!("RUST_GPU_BASE_PATH"), "/script-module.rs"));
    std::env::set_var(
        "RUST_GPU_PATH",
        input.path().unwrap_or_else(|| Path::new("")),
    );
    std::env::set_var("RUST_GPU_SAFE_NAME", input.safe_name());
    std::env::set_var("RUST_GPU_PKG_NAME", input.package_name());
    std::env::set_var("RUST_GPU_BASE_PATH", input.base_path());

    let action = decide_action_for(&input, &args)?;
    info!("action: {:?}", action);

    generate_package(&action)?;

    // Once we're done, clean out old packages from the cache.
    let _defer_clear = {
        Defer::<_, MainError>::new(move || {
            if args.clear_cache {
                // Do nothing if cache was cleared explicitly.
            } else {
                clean_cache(consts::MAX_CACHE_AGE_MS)?;
            }
            Ok(())
        })
    };

    action.execute_command();
    Ok(0)
}

/**
Clean up the cache folder.

Looks for all folders whose metadata says they were created at least `max_age` in the past and kills them dead.
*/
fn clean_cache(max_age: u128) -> MainResult<()> {
    info!("cleaning cache with max_age: {:?}", max_age);

    if max_age == 0 {
        info!("max_age is 0, clearing binary cache...");
        let cache_dir = platform::binary_cache_path();
        if let Err(err) = fs::remove_dir_all(&cache_dir) {
            error!("failed to remove binary cache {:?}: {}", cache_dir, err);
        }
    }

    let cutoff = platform::current_time() - max_age;
    info!("cutoff:     {:>20?} ms", cutoff);

    let cache_dir = platform::generated_projects_cache_path();
    for child in fs::read_dir(cache_dir)? {
        let child = child?;
        let path = child.path();
        if path.is_file() {
            continue;
        }

        info!("checking: {:?}", path);

        let remove_dir = || {
            let meta_mtime = platform::dir_last_modified(&child);
            info!("meta_mtime: {:>20?} ms", meta_mtime);

            meta_mtime <= cutoff
        };

        if remove_dir() {
            info!("removing {:?}", path);
            if let Err(err) = fs::remove_dir_all(&path) {
                error!("failed to remove {:?} from cache: {}", path, err);
            }
        }
    }
    info!("done cleaning cache.");
    Ok(())
}

// Generate a package from the input.
fn generate_package(action: &InputAction) -> MainResult<()> {
    info!("creating pkg dir...");
    fs::create_dir_all(&action.pkg_path)?;
    let cleanup_dir: Defer<_, MainError> = Defer::new(|| {
        if action.using_cache {
            // Only cleanup on failure if we are using the shared package
            // cache, and not when the user has specified the package path
            // (since that would risk removing user files).
            info!("cleaning up cache directory {:?}", &action.pkg_path);
            fs::remove_dir_all(&action.pkg_path)?;
        }
        Ok(())
    });

    info!("generating Cargo package...");
    let mani_path = action.manifest_path();

    overwrite_file(&mani_path, &action.manifest)?;

    info!("disarming pkg dir cleanup...");
    cleanup_dir.disarm();

    Ok(())
}

/**
This represents what to do with the input provided by the user.
*/
#[derive(Debug)]
struct InputAction {
    /// Always show cargo output?
    cargo_output: bool,

    /// Directory where the package should live.
    pkg_path: PathBuf,

    /**
    Is the package directory in the cache?

    Currently, this can be inferred from `emit_metadata`, but there's no *intrinsic* reason they should be tied together.
    */
    using_cache: bool,

    /// If script should be built in debug mode.
    debug: bool,

    /// The package manifest contents.
    manifest: String,

    // Path of the built spir-v file
    spirv_output_path: String,

    // The rust-gpu spir-v targert
    target: String,
}

impl InputAction {
    fn manifest_path(&self) -> PathBuf {
        self.pkg_path.join("Cargo.toml")
    }

    fn execute_command(&self) {
        let mut current_exe_path_buf = std::env::current_exe().unwrap();
        current_exe_path_buf.pop();
        let current_exe_path = current_exe_path_buf.to_str().unwrap();
        let toolchain_path = format!("{current_exe_path}/../share/toolchain");
        let librustc_codegen_spirv_path =
            format!("{current_exe_path}/../lib/librustc_codegen_spirv.so");
        let rustc_path = format!("{toolchain_path}/bin/rustc");
        let cargo_path = format!("{toolchain_path}/bin/cargo");
        let mut cmd = Command::new(cargo_path);

        cmd.arg("build");
        cmd.arg("--message-format=json-render-diagnostics");

        // rust-gpu flags: https://embarkstudios.github.io/rust-gpu/book/writing-shader-crates.html
        // TODO: Default, but take optional from cmdline arg
        cmd.arg("--target");
        cmd.arg(&self.target);
        cmd.arg("-Zbuild-std=core");
        cmd.arg("-Zbuild-std-features=compiler-builtins-mem");
        cmd.env_clear();
        cmd.env("RUSTC", rustc_path);
        cmd.env(
            "RUSTFLAGS",
            format!(
                "-Zcodegen-backend={librustc_codegen_spirv_path} \
        -Zbinary-dep-depinfo \
        -Csymbol-mangling-version=v0 \
        -Zcrate-attr=feature(register_tool) \
        -Zcrate-attr=register_tool(rust_gpu) \
        -Coverflow-checks=off \
        -Cdebug-assertions=off \
        -Zinline-mir=off"
            ),
        );

        if !self.cargo_output {
            cmd.arg("-q");
        }

        cmd.current_dir(&self.pkg_path);

        if platform::force_cargo_color() {
            cmd.arg("--color").arg("always");
        }

        let cargo_target_dir = format!("{}", platform::binary_cache_path().display(),);
        cmd.arg("--target-dir");
        cmd.arg(cargo_target_dir);

        if !self.debug {
            cmd.arg("--release");
        }

        let build_output = cmd
            .stderr(std::process::Stdio::inherit())
            // .current_dir(&builder.path_to_crate)
            .output()
            .expect("Failed to execute cargo build");
        if build_output.status.code() != Some(0) {
            eprintln!("Could not execute cargo");
            std::process::exit(1);
        }
        let stdout = String::from_utf8(build_output.stdout).unwrap();
        let built_spirv_path = match build::parse_metadata_from_stdout(&stdout) {
            Ok(metadata) => metadata,
            Err(error) => {
                eprintln!("--- build output ---\n{stdout}");
                eprintln!("--- error ---\n{error:?}");
                std::process::exit(1);
            }
        };

        if self.spirv_output_path == "-" {
            let bytes = std::fs::read(built_spirv_path).unwrap();
            std::io::stdout().write_all(&bytes).unwrap();
        } else {
            std::fs::copy(built_spirv_path, &self.spirv_output_path).unwrap();
        }
    }
}

/**
For the given input, this constructs the package metadata and checks the cache to see what should be done.
*/
fn decide_action_for(input: &Input, args: &Args) -> MainResult<InputAction> {
    let input_id = input.compute_id();
    info!("id: {:?}", input_id);

    let pkg_name = input.package_name();
    let bin_name = format!("{}_{}", &*pkg_name, input_id.to_str().unwrap());

    let (pkg_path, using_cache) = args
        .pkg_path
        .as_ref()
        .map(|p| (p.into(), false))
        .unwrap_or_else(|| {
            let cache_path = platform::generated_projects_cache_path();
            (cache_path.join(&input_id), true)
        });
    info!("pkg_path: {:?}", pkg_path);
    info!("using_cache: {:?}", using_cache);

    let base_path = match &args.base_path {
        Some(path) => Path::new(path).into(),
        None => input.base_path(),
    };

    let (mani_str, _script_path) = manifest::split_input(input, &base_path, &bin_name)?;

    let spirv_output_path = args.output_path.to_owned().unwrap_or_else(|| {
        let mut path = PathBuf::from(args.script.clone().unwrap());
        path.set_extension("spv");
        path.to_str().unwrap().to_owned()
    });

    Ok(InputAction {
        cargo_output: args.cargo_output,
        debug: args.debug,
        manifest: mani_str,
        pkg_path,
        spirv_output_path,
        target: args.target.clone(),
        using_cache,
    })
}

/// Attempts to locate the script specified by the given path.
fn find_script(path: &Path) -> Option<(PathBuf, fs::File)> {
    if let Ok(file) = fs::File::open(path) {
        return Some((path.into(), file));
    }

    if path.extension().is_none() {
        for &ext in &["ers", "rs"] {
            let path = path.with_extension(ext);
            if let Ok(file) = fs::File::open(&path) {
                return Some((path, file));
            }
        }
    }

    None
}

/**
Represents an input source for a script.
*/
#[derive(Clone, Debug)]
pub enum Input {
    /**
    The input is a script file.

    The tuple members are: the name, absolute path, script contents.
    */
    File(String, PathBuf, String),
}

impl Input {
    /**
    Return the path to the script, if it has one.
    */
    pub fn path(&self) -> Option<&Path> {
        use crate::Input::*;

        match self {
            File(_, path, _) => Some(path),
        }
    }

    /**
    Return the "safe name" for the input.  This should be filename-safe.

    Currently, nothing is done to ensure this, other than hoping *really hard* that we don't get fed some excessively bizarre input filename.
    */
    pub fn safe_name(&self) -> &str {
        use crate::Input::*;

        match self {
            File(name, _, _) => name,
        }
    }

    /**
    Return the package name for the input.  This should be a valid Rust identifier.
    */
    pub fn package_name(&self) -> String {
        let name = self.safe_name();
        let mut r = String::with_capacity(name.len());

        for (i, c) in name.chars().enumerate() {
            match (i, c) {
                (0, '0'..='9') => {
                    r.push('_');
                    r.push(c);
                }
                (_, '0'..='9') | (_, 'a'..='z') | (_, '_') | (_, '-') => {
                    r.push(c);
                }
                (_, 'A'..='Z') => {
                    // Convert uppercase characters to lowercase to avoid `non_snake_case` warnings.
                    r.push(c.to_ascii_lowercase());
                }
                (_, _) => {
                    r.push('_');
                }
            }
        }

        r
    }

    /**
    Base directory for resolving relative paths.
    */
    pub fn base_path(&self) -> PathBuf {
        match self {
            Self::File(_, path, _) => path
                .parent()
                .expect("couldn't get parent directory for file input base path")
                .into(),
        }
    }

    // Compute the package ID for the input.
    // This is used as the name of the cache folder into which the Cargo package
    // will be generated.
    pub fn compute_id(&self) -> OsString {
        use crate::Input::*;

        match self {
            File(_, path, _) => {
                let mut hasher = Sha1::new();

                // Hash the path to the script.
                hasher.update(&*path.to_string_lossy());
                let mut digest = format!("{:x}", hasher.finalize());
                digest.truncate(consts::ID_DIGEST_LEN_MAX);

                let mut id = OsString::new();
                id.push(&*digest);
                id
            }
        }
    }
}

// Overwrite a file if and only if the contents have changed.
fn overwrite_file(path: &Path, content: &str) -> MainResult<()> {
    debug!("overwrite_file({:?}, _)", path);
    let mut existing_content = String::new();
    match fs::File::open(path) {
        Ok(mut file) => {
            file.read_to_string(&mut existing_content)?;
            if existing_content == content {
                debug!("Equal content");
                return Ok(());
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Continue
        }
        Err(e) => {
            return Err(error::MainError::Io(e));
        }
    }

    debug!(".. files differ");
    let dir = path.parent().ok_or("The given path should be a file")?;
    let mut temp_file = tempfile::NamedTempFile::new_in(dir)?;
    temp_file.write_all(content.as_bytes())?;
    temp_file.flush()?;
    temp_file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}

#[test]
fn test_package_name() {
    let input = Input::File(
        "Script".to_string(),
        Path::new("path").into(),
        "script".to_string(),
    );
    assert_eq!("script", input.package_name());
    let input = Input::File(
        "1Script".to_string(),
        Path::new("path").into(),
        "script".to_string(),
    );
    assert_eq!("_1script", input.package_name());
}
