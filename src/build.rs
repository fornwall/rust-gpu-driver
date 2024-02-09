// TODO: Copied from spirv-builder
pub use rustc_codegen_spirv_types::{CompileResult, ModuleResult};
use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

const ARTIFACT_SUFFIX: &str = ".spv.json";

pub(crate) fn parse_metadata_from_stdout(out: &str) -> Result<PathBuf, SpirvBuilderError> {
    let last = out
        .lines()
        .filter_map(|line| {
            if let Ok(line) = serde_json::from_str::<RustcOutput>(line) {
                Some(line)
            } else {
                // Pass through invalid lines
                println!("{line}");
                None
            }
        })
        .filter(|line| line.reason == "compiler-artifact")
        .last()
        .expect("Did not find output file in rustc output");

    let mut filenames = last
        .filenames
        .unwrap()
        .into_iter()
        .filter(|v| v.ends_with(ARTIFACT_SUFFIX));
    let filename = filenames.next().unwrap();
    assert_eq!(
        filenames.next(),
        None,
        "build had multiple `{ARTIFACT_SUFFIX}` artifacts"
    );

    parse_metadata_file(&filename.into())
}

#[derive(Deserialize)]
struct RustcOutput {
    reason: String,
    filenames: Option<Vec<String>>,
}

#[derive(Debug)]
pub enum SpirvBuilderError {
    MetadataFileMissing(std::io::Error),
    MetadataFileMalformed(serde_json::Error),
}

pub(crate) fn parse_metadata_file(at: &PathBuf) -> Result<PathBuf, SpirvBuilderError> {
    let metadata_contents = File::open(at).map_err(SpirvBuilderError::MetadataFileMissing)?;
    let metadata: CompileResult = serde_json::from_reader(BufReader::new(metadata_contents))
        .map_err(SpirvBuilderError::MetadataFileMalformed)?;
    match metadata.module {
        ModuleResult::SingleModule(spirv_module) => Ok(spirv_module),
        ModuleResult::MultiModule(_) => {
            panic!("Multiple modules");
        }
    }
}
