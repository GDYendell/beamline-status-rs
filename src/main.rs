use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use tabled::{Table, Tabled};

static REDIRECT_TABLE: &str = "/dls_sw/prod/etc/redirector/redirect_table";
static DLS_SW_WORK: &str = "/dls_sw/work";

#[derive(Debug, Tabled)]
struct IOC {
    name: String,
    version: String,
}

fn main() {
    let ioc_versions = match find_ioc_versions() {
        Ok(ioc_paths) => ioc_paths,
        Err(e) => {
            eprintln!("Error reading redirect table: {}", e);
            std::process::exit(1);
        }
    };

    let iocs = ioc_versions
        .into_iter()
        .map(|(name, version)| IOC { name, version })
        .collect::<Vec<_>>();

    let table = Table::new(iocs).to_string();

    println!("{}", table);
}

fn find_ioc_versions() -> Result<HashMap<String, String>, Box<dyn Error>> {
    let redirect_re: Regex = Regex::new(r"^(?<name>BL07I\S+)\s+(?<path>\S+)$").unwrap();
    let dls_version_re: Regex = Regex::new(r"\/(?<version>\d+(?:-\d+)+)\/").unwrap();

    let mut ioc_versions = HashMap::new();
    for line in BufReader::new(File::open(REDIRECT_TABLE)?).lines() {
        if let Some(captures) = redirect_re.captures(&line?) {
            let name = captures["name"].to_string();
            let path = &captures["path"];

            if let Some(captures) = dls_version_re.captures(&path) {
                ioc_versions.insert(name, captures["version"].to_string());
            } else if path.starts_with(DLS_SW_WORK) {
                ioc_versions.insert(name, "WORK".to_string());
            } else {
                ioc_versions.insert(name, "WORK?".to_string());
            }
        }
    }

    Ok(ioc_versions)
}
