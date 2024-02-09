// From https://gist.github.com/MiSawa/6e9e3c400803e7f4ae4d190d4477b4b8
// Context: https://github.com/fornwall/rust-script/issues/11#issuecomment-1516408804
use std::{collections::{HashMap, HashSet}, io::Write, path::{Path, PathBuf}};

use clap::Parser;
use project_model::{CargoConfig, ProjectManifest, ProjectWorkspace, RustLibSource, Sysroot};
use serde::Serialize;

#[derive(Serialize, Debug)]
struct RustProject {
    sysroot: PathBuf,
    sysroot_src: PathBuf,
    crates: Vec<Crate>,
}
#[derive(Serialize, Debug)]
struct Crate {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    display_name: Option<String>,
    root_module: PathBuf,
    edition: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    version: Option<String>,
    deps: Vec<Dep>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    cfg: Vec<String>,

    is_proc_macro: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    proc_macro_dylib_path: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    repository: Option<String>,
}
#[derive(Serialize, Debug)]
struct Dep {
    #[serde(rename = "crate")]
    krate: usize,
    name: String,
}

fn edit_project(shader_path: &Path, project_dir: PathBuf) -> Result<()> {
    let sysroot = Sysroot::discover(&project_dir, &Default::default()).unwrap();

    let manifest = ProjectManifest::from_manifest_file(project_dir.join("Cargo.toml")).unwrap();

    let mut config = CargoConfig::default();
    config.sysroot = Some(RustLibSource::Path(sysroot.root().to_path_buf()));
    config.rustc_source = Some(RustLibSource::Discover);

    let ws = ProjectWorkspace::load(manifest, &config, &|_| {}).unwrap();

    let mut path_to_id = HashMap::new();
    let mut id_to_path = HashMap::new();
    let (crate_graph, proc_macro_paths) = ws.to_crate_graph(
        &mut |path| {
            let path = if path.parent() == Some(&project_dir) {
                // Translate the script path!
                shader_path.clone()
            } else {
                path.to_owned()
            };
            let n = path_to_id.len() as u32;
            let ret = vfs::FileId(path_to_id.entry(path.to_owned()).or_insert(n).clone());
            id_to_path
                .entry(ret.clone())
                .or_insert_with(|| path.to_owned());
            Some(ret)
        },
        &Default::default(),
    );

    let mut crate_ids = vec![];
    let mut crate_id_to_index = HashMap::new();
    for id in crate_graph.iter() {
        crate_ids.push(id);
        crate_id_to_index.insert(id, crate_id_to_index.len());
    }

    let cfg_keys_to_ignore: HashSet<&'static str> = HashSet::from_iter([
        "debug_assertions",
        "panic",
        "target_abi",
        "target_arch",
        "target_endian",
        "target_env",
        "target_family",
        "target_feature",
        "target_has_atomic",
        "target_has_atomic_equal_alignment",
        "target_has_atomic_load_store",
        "target_os",
        "target_pointer_width",
        "target_thread_local",
        "target_vendor",
        "unix",
        "windows",
    ]);

    let mut crates = vec![];
    for id in crate_ids {
        let data = &crate_graph[id];
        let display_name = data
            .display_name
            .as_ref()
            .map(|name| name.canonical_name().to_owned());

        let root_module = id_to_path[&data.root_file_id].clone().into();
        let edition = match data.edition {
            base_db::Edition::Edition2015 => "2015",
            base_db::Edition::Edition2018 => "2018",
            base_db::Edition::Edition2021 => "2021",
        }
        .to_owned();
        let deps = data
            .dependencies
            .iter()
            .map(|dep| Dep {
                krate: crate_id_to_index[&dep.crate_id],
                name: dep.name.to_string(),
            })
            .collect();
        let cfg: Vec<_> = data
            .cfg_options
            .get_cfg_keys()
            .filter(|key| !cfg_keys_to_ignore.contains(key.as_str()))
            .flat_map(|key| {
                data.cfg_options
                    .check(&cfg::CfgExpr::Atom(cfg::CfgAtom::Flag(key.clone())))
                    .filter(|x| *x)
                    .map(|_| format!("{key}"))
                    .into_iter()
                    .chain(
                        data.cfg_options
                            .get_cfg_values(key)
                            // TODO: Escape?
                            .map(move |value| format!(r#"{key}="{value}""#)),
                    )
            })
            .collect();
        let proc_macro_dylib_path = proc_macro_paths
            .get(&id)
            .and_then(|v| v.as_ref().ok().map(|v| v.1.clone().into()));
        let repository = match &data.origin {
            base_db::CrateOrigin::Local { repo, .. }
            | base_db::CrateOrigin::Library { repo, .. } => repo.clone(),
            _ => None,
        };

        crates.push(Crate {
            display_name,
            root_module,
            edition,
            version: data.version.clone(),
            deps,
            cfg,
            is_proc_macro: data.is_proc_macro,
            proc_macro_dylib_path,
            repository,
        });
    }

    let project = RustProject {
        sysroot: sysroot.root().to_owned().into(),
        sysroot_src: sysroot.src_root().to_owned().into(),
        crates,
    };

    let json = serde_json::to_string_pretty(&project).expect("unable to serialize to json");
    let rust_project_json_path= project_dir.join("rust-project.json");
    let mut rust_project_json_file = std::fs::OpenOptions::new()
        .create(true) // To create a new file
        .write(true)
        .open(rust_project_json_path)
        .expect("Could not open rust-project.json");
    rust_project_json_file.write_all(json.as_bytes());

    Ok(())
}
