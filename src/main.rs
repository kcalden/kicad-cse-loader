use lazy_static::lazy_static;
use serde::{Deserialize,Serialize};
use std::fs::{canonicalize, DirBuilder, File, read_to_string, DirEntry};
use regex::Regex;
use std::collections::HashMap;
use zip::ZipArchive;
use std::io::{Read, Write};
use std::time::Instant;

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    main_lib_location: String,
    main_lib_name: String,
    download_folder: String,
}

#[derive(Debug)]
struct Component {
    lib_def: String,
    dcm_def: String,
    model_file: Option<String>,
    footprint_file: String,
    footprint_name: String
}

const CONFIG_PATH: &str = "./loader.toml";

const LIB_HEADER: &str = r"EESchema-LIBRARY Version 2.4";


const DCM_HEADER: &str = r"EESchema-DOCLIB  Version 2.0";

const LIB_FOOTER: &str = "#End Doc Lib";

fn main() -> std::io::Result<()> {
    // LOAD CONFIG
    let config_path = canonicalize(CONFIG_PATH)?;
    let config_contents = std::fs::read_to_string(config_path)?;
    // config contains information for merging libraries
    let config: Config = toml::from_str(&config_contents)?;
    
    // FIND LIBRARY FILES DOWNLOADED FROM COMPONENT SEARCH ENGINE
    // libs is an iterator through the library files found in the download folder
    // let cse_libs = std::fs::read_dir(canonicalize(&config.download_folder)?)?.filter(|entry_res| {
    //     if let Ok(_) = entry_res {
    //         return true
    //     }else{
    //         return false
    //     }
    // })
    // .map(|entry_res| { entry_res.unwrap()})
    // .filter(|dir_entry| {
    //     let file_name = dir_entry.file_name();
    //     let file_name = file_name.to_str().unwrap();
    //     if file_name.starts_with("LIB_") && file_name.ends_with(".zip") {
    //         return true
    //     }else{return false}
    // });
    
    // ATTEMPT TO INIT LIBRARY IF IT'S NOT FOUND
    let pretty_folder = format!("{}{}.pretty",config.main_lib_location, config.main_lib_name);
    let shapes_folder = format!("{}{}.3dshapes",config.main_lib_location, config.main_lib_name);
    let lib_file_path = format!("{}{}.lib",config.main_lib_location, config.main_lib_name);
    let dcm_file_path = format!("{}{}.dcm",config.main_lib_location, config.main_lib_name);
    
    // Create .pretty folder
    DirBuilder::new()
    .recursive(true)
    .create(&pretty_folder)?;

    // Create .3dshapes folder
    DirBuilder::new()
    .recursive(true)
    .create(&shapes_folder)?;

    // Attempt to read the lib file
    let lib_file_contents = match read_to_string(&lib_file_path) {
        Ok(contents) => contents,
        Err(_) => String::from(LIB_HEADER),
    };
    
    // Pull defs from main library
    let mut main_lib_defs = get_lib_defs(&lib_file_contents);

    // Attempt to read the dcm file
    let dcm_file_contents = match read_to_string(&dcm_file_path) {
        Ok(contents) => contents,
        Err(_) => String::from(DCM_HEADER),
    };

    let mut main_dcm_defs = get_dcm_defs(&dcm_file_contents);

    let stopwatch = Instant::now();
    // Grab all data for the new components and merge them with ours
    let new_components = get_component_archives(&config.download_folder);

    // Push component data
    for (component_name,component) in new_components {
        println!("{} -> {}", component_name, config.main_lib_name);
        // Write lib definition
        main_lib_defs.insert(component_name.clone(), component.lib_def);
        // Write dcm definition
        main_dcm_defs.insert(component_name.clone(), component.dcm_def);

        // Write model file
        if let Some(model_file) = component.model_file {
            let mut m = File::create(format!("{}/{}.stp", &shapes_folder, &component_name))?;
            m.write(model_file.as_bytes());
        }

        // Write footprint file
        let mut f = File::create(format!("{}/{}.kicad_mod", &pretty_folder, &component.footprint_name))?;
        f.write(component.footprint_file.as_bytes());
    }

    // Create lib file
    let mut new_lib_file = String::new();
    new_lib_file.push_str(LIB_HEADER);
    new_lib_file.push_str("\n#encoding utf-8\n");
    new_lib_file.push_str("#\n");
    for (component_name, lib_def) in main_lib_defs {
        new_lib_file.push_str(format!("# {}\n", component_name).as_str());
        new_lib_file.push_str("#\n");
        new_lib_file.push_str(&lib_def);
        new_lib_file.push_str("\n#\n");
    }
    new_lib_file.push_str("#End Library");

    let mut lib_file = File::create(lib_file_path)?;
    lib_file.write(new_lib_file.as_bytes());

    let mut new_dcm_file = String::new();
    new_dcm_file.push_str(DCM_HEADER);
    new_lib_file.push_str("\n#\n");
    for (component_name, lib_def) in main_dcm_defs {
        // new_dcm_file.push_str("#\n");
        new_dcm_file.push_str(&lib_def);
        new_dcm_file.push_str("\n#\n");
    }
    new_dcm_file.push_str("#End Doc Library");

    let mut dcm_file = File::create(dcm_file_path)?;
    dcm_file.write(new_dcm_file.as_bytes());

    println!("Merged in {} ms",stopwatch.elapsed().as_millis());
    Ok(())
}

fn get_lib_defs(contents: &str) -> HashMap<String,String> {
    lazy_static! {
        static ref LIB_DEF_RE: Regex = Regex::new(r"(?s)(?P<definition>DEF\s+(?P<name>.*?)\s+.*?ENDDEF)+").unwrap();
    }
    let mut lib_defs = HashMap::new();
    for cap in LIB_DEF_RE.captures_iter(contents) {
        let name = String::from(cap.name("name").unwrap().as_str());
        let def = String::from(cap.name("definition").unwrap().as_str());
        lib_defs.insert(name, def);
    }
    lib_defs
}

fn get_dcm_defs(contents: &str) -> HashMap<String,String> {
    lazy_static! {
        static ref LIB_DEF_RE: Regex = Regex::new(r"(?s)(?P<definition>\$CMP\s+(?P<name>.*?)\s+.*?\$ENDCMP)+").unwrap();
    }
    let mut lib_defs = HashMap::new();
    for cap in LIB_DEF_RE.captures_iter(contents) {
        let name = String::from(cap.name("name").unwrap().as_str());
        let def = String::from(cap.name("definition").unwrap().as_str());
        lib_defs.insert(name, def);
    }
    lib_defs
}

fn get_component_archives(download_folder: &str) -> HashMap<String, Component>{
    use rayon::prelude::*;
    lazy_static! {
        static ref FOOTPRINT_FILE_RE: Regex = Regex::new(r".+/(?P<footprint_name>.*)\.kicad_mod").unwrap();
    }
    let cse_libs: Vec<DirEntry> = std::fs::read_dir(canonicalize(&download_folder).unwrap()).unwrap().filter(|entry_res| {
        if let Ok(_) = entry_res {
            return true
        }else{
            return false
        }
    })
    .map(|entry_res| { entry_res.unwrap()})
    .filter(|dir_entry| {
        let file_name = dir_entry.file_name();
        let file_name = file_name.to_str().unwrap();
        if file_name.starts_with("LIB_") && file_name.ends_with(".zip") {
            return true
        }else{return false}
    }).collect();

    cse_libs.par_iter().map(|dir_entry| {
        let mut component = Component {
            dcm_def: String::new(),
            lib_def: String::new(),
            footprint_file: String::new(),
            footprint_name: String::new(),
            model_file: None
        };

        let component_name = dir_entry.file_name().to_str().unwrap()
                                                .replace("LIB_", "")
                                                .replace(".zip", "");
        
        let mut component_archive = ZipArchive::new(File::open(dir_entry.path()).unwrap()).unwrap();
        // Grab lib file
        {   
            let mut comp_lib_file = component_archive.by_name(format!("{}/KiCad/{}.lib",component_name,component_name).as_str()).unwrap();
            let mut lib_file_buf = String::new();
            comp_lib_file.read_to_string(&mut lib_file_buf);
            let lib_def = get_lib_defs(&lib_file_buf);
            let lib_def = lib_def.get(&component_name).unwrap();
            component.lib_def = lib_def.clone();
        }

        // Grab document file
        {   
            let mut comp_dcm_file = component_archive.by_name(format!("{}/KiCad/{}.dcm",component_name,component_name).as_str()).unwrap();
            let mut dcm_file_buf = String::new();
            comp_dcm_file.read_to_string(&mut dcm_file_buf);
            let dcm_def = get_dcm_defs(&dcm_file_buf);
            let dcm_def = dcm_def.get(&component_name).unwrap();
            component.dcm_def = dcm_def.clone();
        }
        
        // Grab model if it's available
        {
            let mut comp_stp_file = component_archive.by_name(format!("{}/3D/{}.stp",component_name,component_name).as_str());
            let mut model_file_buffer = String::new();
            if let Ok(file) = &mut comp_stp_file {
                file.read_to_string(&mut model_file_buffer);
                component.model_file = Some(model_file_buffer);
            }
        }

        // Find the footprint file because it could be named anything
        let mut footprint_file_name = String::new();
        {
            for file_name in component_archive.file_names() {
                if file_name.ends_with(".kicad_mod") {
                    for cap in FOOTPRINT_FILE_RE.captures_iter(&file_name) {
                        component.footprint_name = String::from(cap.name("footprint_name").unwrap().as_str());
                    }
                    footprint_file_name.push_str(file_name);
                }
            }

            let mut comp_footprint_file = component_archive.by_name(&footprint_file_name).unwrap();
            comp_footprint_file.read_to_string(&mut component.footprint_file);
        }
        (component_name,component)
    }).collect()
}