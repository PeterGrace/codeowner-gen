mod lib;

#[cfg(test)]
mod tests;

use log::{debug};
use env_logger;
use std::fs::File;
use clap::{App, load_yaml};
use std::io::Read;
use crate::lib::{CodeOwners, CodeOwner, Owner};
use serde_yaml::Error;
use anyhow::{Result, bail};
use itertools::Itertools;

static COMPRESSED_DEPENDENCY_LIST: &[u8] = auditable::inject_dependency_list!();

fn main() -> Result<()> {
    let yaml = load_yaml!("clap_definition.yaml");
    let matches = App::from_yaml(yaml).get_matches();
    match matches.is_present("debug") {
        true => { std::env::set_var("RUST_LOG", "debug") }
        false => ()
    };
    env_logger::init();
    debug!(
        "codeowner-gen cargover:{} githash:{} auditable_count:{}",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        COMPRESSED_DEPENDENCY_LIST.len()
    );
    let filename = matches.value_of("input_file").unwrap_or("codeowners.yaml");
    let mut file = File::open(filename)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let mut code_owners = match serde_yaml::from_str::<CodeOwners>(contents.as_str()) {
        Ok(v) => v,
        Err(e) => bail!("Unable to parse config file: {:#?}", e)
    };
    let mut longest_path = 0;
    let mut grouped = false;
    // for mvp, find longest path first...
    for code_owner in &code_owners.entries {
        if code_owner.path.len() > longest_path {
            longest_path = code_owner.path.len()
        }
        match code_owner.group {
            Some(_) => { if grouped==false { grouped=true;}},
            None => ()
        };
    }
    let mut sorted_output: Vec<CodeOwners> = vec![];
    if grouped == true {
        sorted_output = code_owners
            .iter()
            .group_by(|group| group.group.clone())
            .map()

    }
    else {
        sorted_output = code_owners
            .iter()
            .group_by()

    }
    Ok(())
}
