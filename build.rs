use eyre::{eyre, Result};
use std::{env, fs, path::PathBuf};

use ethers::{
    contract::Abigen,
    solc::{ConfigurableArtifacts, Project, ProjectCompileOutput, ProjectPathsConfig, Solc},
};
use inflector::Inflector;
use semver::{Version, VersionReq};

const SOLC_VERSION_REQ: &str = "^0.8.0";
const COMPILE_PATH: &str = "test/contracts";

fn main() -> Result<()> {
    if env::var_os("CARGO_FEATURE_TXGEN").is_none() {
        return Ok(());
    }

    let contracts_to_bind = vec!["Store", "Factory"];

    let base_dir = std::env::current_dir()?;
    let build_dir = mkdir(base_dir.join("build"));
    let bindings_dir = mkdir(base_dir.join("src/bindings"));

    let output = compile(base_dir.join(COMPILE_PATH))?;

    let mut bindings = String::new();
    for name in contracts_to_bind {
        let contract = output.find(name).ok_or(eyre!(
            "Could Not bind contract {}. Compiler output not found.",
            name
        ))?;

        // write bytecode to build dir if binding a non-abstract contract
        if let Some(bin) = &contract.bytecode {
            fs::write(
                &build_dir.join(name.clone().to_snake_case().to_owned() + ".bin"),
                hex::encode(bin.object.clone()),
            )?;
        }

        // generate bindings from the abi
        let abi = serde_json::to_string(
            contract
                .abi
                .as_ref()
                .expect("tried to bind a contract with no abi"),
        )?;

        let mod_name = name.to_snake_case();
        Abigen::new(name, abi)
            .map_err(|e| eyre!("new abigen failure: {}", e))?
            .generate()
            .map_err(|e| eyre!("abigen failure: {}", e))?
            .write_to_file(bindings_dir.join(mod_name.clone() + ".rs"))
            .map_err(|e| eyre!("failed to write bindings: {}", e))?;

        bindings.push_str(&format!("pub mod {};\n", mod_name));
    }

    fs::write(bindings_dir.join("mod.rs"), bindings)?;

    // Pass build_dir to env as SOLC_BUILD_DIR
    println!(
        "cargo:rustc-env=SOLC_BUILD_DIR={}",
        build_dir.into_os_string().into_string().unwrap()
    );

    Ok(())
}

fn compile(dir: PathBuf) -> Result<ProjectCompileOutput<ConfigurableArtifacts>> {
    let solc = Solc::default();
    check_solc(solc.version().expect("No solc version"));

    let paths = ProjectPathsConfig::builder().sources(dir).build()?;
    let project = Project::builder()
        .paths(paths)
        .solc(solc)
        .no_artifacts()
        .build()?;

    // tell cargo to rerun build script if contracts change
    project.rerun_if_sources_changed();

    let output = project.compile()?;
    if output.has_compiler_errors() {
        eyre::bail!(output.to_string())
    } else {
        Ok(output)
    }
}

fn check_solc(version: Version) {
    let req = VersionReq::parse(SOLC_VERSION_REQ).expect("Cannot parse SOLC_VERSION_REQ");
    if !req.matches(&version) {
        println!("cargo:warning=solc version mismatch. Using local solc executable, version: {}. Expected: {}", version, req.to_string());
    }
}

fn mkdir(dir: PathBuf) -> PathBuf {
    if !dir.exists() {
        fs::create_dir(&dir).expect(&format!("could not create dir: {}", dir.to_string_lossy()));
    }
    dir
}
