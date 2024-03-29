mod lib;

#[cfg(test)]
mod tests;

use log::{debug};
use env_logger;
use std::fs::{File, OpenOptions};
use clap::{App, load_yaml};
use std::io::{Read, Write};
use crate::lib::CodeOwners;
use anyhow::{Result, bail};

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
            Some(_) => { if !grouped { grouped = true; } }
            None => ()
        };
    }
    code_owners.entries.sort_by_key(|x| x.path.clone());
    if grouped {
        code_owners.entries.sort_by_key(|x| x.group.clone());
    };


    // And now, to write the file.
    let mut fd = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("CODEOWNERS")?;
    fd.write_all(format!("# Generated by codeowner-gen v{}/{}\n#\n",
                     env!("CARGO_PKG_VERSION"),
                     env!("GIT_HASH")).as_bytes())?;
    fd.write_all("####### BEGIN UNGROUPED\n".as_bytes())?;
    let mut last_group = String::new();
    for co in code_owners.entries {
        let mut owners = String::new();
        for o in co.owners {
            owners = format!("{} {}", owners, o);
        }
        if grouped {
            if co.group.clone().is_some() {
                if last_group.is_empty() {
                    last_group = co.group.clone().unwrap();
                    fd.write_all(format!("####### BEGIN GROUP {}\n", co.group.clone().unwrap().to_ascii_uppercase()).as_bytes())?;
                }
                if co.group.clone().unwrap() != last_group
                {
                    fd.write_all(format!("### END GROUP {}\n", last_group.to_ascii_uppercase()).as_bytes())?;
                    fd.write_all(format!("####### BEGIN GROUP {}\n", co.group.clone().unwrap().to_ascii_uppercase()).as_bytes())?;
                    last_group = co.group.clone().unwrap();
                }
            }
        }
        if co.comment.is_some() {
            fd.write_all(format!("# {}\n", co.comment.unwrap()).as_bytes())?;
        };
        fd.write_all(format!("{:width$} {}\n", co.path, owners, width = longest_path).as_bytes())?;
    }
    Ok(())
}
