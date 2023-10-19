use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use tabled::{Table, Tabled};

static REDIRECT_TABLE: &str = "/dls_sw/prod/etc/redirector/redirect_table";

#[derive(Debug, Tabled)]
struct IOC {
    name: String,
    path: String,
}

fn main() {
    let ioc_paths = match read_redirect_table() {
        Ok(ioc_paths) => ioc_paths,
        Err(e) => {
            eprintln!("Error reading redirect table: {}", e);
            std::process::exit(1);
        }
    };

    let iocs = ioc_paths
        .iter()
        .map(|(name, path)| IOC {
            name: name.to_string(),
            path: path.to_string(),
        })
        .collect::<Vec<_>>();

    let table = Table::new(iocs).to_string();

    println!("{}", table);
}

fn read_redirect_table() -> Result<HashMap<String, String>, Box<dyn Error>> {
    let redirect_re: Regex = Regex::new(r"^(?<name>BL07I\S+)\s+(?<path>\S+)$").unwrap();

    let file = File::open(REDIRECT_TABLE)?;
    let reader = BufReader::new(file);

    let ioc_paths = reader
        .lines()
        .flat_map(|line| match redirect_re.captures(&line.unwrap()) {
            Some(captures) => Some((captures["name"].to_string(), captures["path"].to_string())),
            _ => None,
        })
        .collect::<HashMap<_, _>>();

    Ok(ioc_paths)
}
