use clap::Parser;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::ops::AddAssign;
use tabled::{Table, Tabled};

static REDIRECT_TABLE: &str = "/dls_sw/prod/etc/redirector/redirect_table";
static DLS_SW_WORK: &str = "/dls_sw/work";

/// A fully defined IOC that can be formatted as a table row
#[derive(Debug, Tabled)]
struct IOC {
    name: String,
    description: String,
    version: String,
    builder: bool,
    // platform: String,
    // running: bool,
    // epics_version: String,
    // bl_dependency: String,
    // bl_database: bool,
    // support_module: String,
    // builder_makefile: bool,
    // builder_active: bool,
    // who: String,
    // when: String,
    // server: String,
}

fn read_description(readme_path: String) -> Result<String, Box<dyn Error>> {
    let file = File::open(readme_path)?;
    let reader = BufReader::new(file);

    match reader.lines().next() {
        Some(Ok(line)) => Ok(line),
        _ => Ok("".to_string()),
    }
}

/// A partial representation of an IOC with optional fields that can be populated in stages
#[derive(Debug, Eq, Ord)]
struct PartialIOC {
    name: String,
    description: String,
    version: String,
    builder: bool,
    // platform: String,
    // running: bool,
    // epics_version: String,
    // bl_dependency: String,
    // bl_database: bool,
    // support_module: String,
    // builder_makefile: bool,
    // builder_active: bool,
    // who: String,
    // when: String,
    // server: String,
}

impl PartialIOC {
    fn new(name: String, version: String, readme: String, builder: bool) -> Self {
        PartialIOC {
            name,
            description: read_description(readme).unwrap_or("".to_string()),
            version,
            builder,
        }
    }

    fn from_builder_ioc(name: String, builder_path: String) -> Self {
        let readme = format!("{}/{}_README", builder_path, name);
        PartialIOC::new(name, "BUILDER".to_string(), readme, true)
    }

    fn from_configured_ioc(name: String, path: String, version: String) -> Self {
        let readme = format!("{}/README", path.split("/bin/").next().unwrap().to_string());
        PartialIOC::new(name, version, readme, false)
    }
}

impl PartialEq for PartialIOC {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialOrd for PartialIOC {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.name.cmp(&other.name))
    }
}

impl AddAssign for PartialIOC {
    fn add_assign(&mut self, rhs: Self) {
        self.builder = self.builder | rhs.builder;
    }
}

#[derive(clap::Parser)]
struct Cli {
    /// Beamline to display status for
    beamline: String,
    /// Query pattern for IOCs - defaults to beamline identifier, but can be any
    /// subsection of the IOCs to match, or a regex pattern.
    ///   e.g. 'BL07I', 'BL07I.*EA.*'
    pattern: Option<String>,
}

fn main() {
    let args = Cli::parse();
    let beamline = args.beamline;
    let pattern = match args.pattern {
        Some(pattern) => pattern,
        None => beamline.clone(),
    };

    let mut partial_iocs = match find_configured_iocs(pattern.as_str()) {
        Ok(iocs) => iocs,
        Err(e) => {
            eprintln!("Error reading redirect table: {}", e);
            std::process::exit(1);
        }
    };

    let builder_iocs = match find_builder_iocs(beamline.as_str(), pattern.as_str()) {
        Ok(iocs) => iocs,
        Err(e) => {
            eprintln!("Error reading builder IOCs: {}", e);
            std::process::exit(1);
        }
    };

    for (name, ioc) in builder_iocs {
        if partial_iocs.contains_key(&name) {
            *partial_iocs.get_mut(&name).unwrap() += ioc;
        } else {
            partial_iocs.insert(name, ioc);
        }
    }

    let mut partial_iocs: Vec<&PartialIOC> = partial_iocs.values().collect();

    partial_iocs.sort();

    let iocs: Vec<IOC> = partial_iocs
        .iter()
        .map(|ioc| IOC {
            name: ioc.name.clone(),
            description: ioc.description.clone(),
            version: ioc.version.clone(),
            builder: ioc.builder,
        })
        .collect();

    let table = Table::new(iocs).to_string();

    println!("{}", table);
}

fn find_configured_iocs(query: &str) -> Result<HashMap<String, PartialIOC>, Box<dyn Error>> {
    let redirect_re: Regex =
        Regex::new(format!(r"^(?<name>\S*{query}\S*)\s+(?<path>\S+)$").as_str()).unwrap();
    let dls_version_re: Regex = Regex::new(r"\/(?<version>\d+(?:-\d+)+)\/").unwrap();

    let mut configured_iocs: HashMap<String, PartialIOC> = HashMap::new();
    for line in BufReader::new(File::open(REDIRECT_TABLE)?).lines() {
        if let Some(captures) = redirect_re.captures(&line?) {
            let name = &captures["name"];
            let full_path = &captures["path"];

            let version = if let Some(captures) = dls_version_re.captures(&full_path) {
                captures["version"].to_string()
            } else if full_path.starts_with(DLS_SW_WORK) {
                "WORK".to_string()
            } else {
                "?".to_string()
            };

            configured_iocs.insert(
                name.to_string(),
                PartialIOC::from_configured_ioc(name.to_string(), full_path.to_string(), version),
            );
        }
    }

    Ok(configured_iocs)
}

fn find_builder_iocs(
    beamline: &str,
    pattern: &str,
) -> Result<HashMap<String, PartialIOC>, Box<dyn Error>> {
    let builder_path = format!(
        "/dls_sw/work/R3.14.12.7/support/{}-BUILDER/etc/makeIocs",
        beamline
    )
    .to_string();
    let ioc_re: Regex = Regex::new(pattern).unwrap();

    let mut builder_iocs: HashMap<String, PartialIOC> = HashMap::new();
    for file in std::fs::read_dir(&builder_path).unwrap() {
        let file_name = file.unwrap().file_name().into_string().unwrap();
        if file_name.ends_with(".xml") {
            let name = file_name.split(".").next().unwrap().to_string();
            if ioc_re.is_match(&name) {
                builder_iocs.insert(
                    name.clone(),
                    PartialIOC::from_builder_ioc(name, builder_path.clone()),
                );
            }
        }
    }

    Ok(builder_iocs)
}
