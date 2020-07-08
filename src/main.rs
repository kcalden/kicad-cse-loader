use serde::{Deserialize,Serialize};
use std::fs::{canonicalize, DirBuilder, File, read_to_string};
use regex::Regex;

#[derive(Deserialize, Serialize, Debug)]
struct Config {
    main_lib_location: String,
    main_lib_name: String,
    download_folder: String,
}

const CONFIG_PATH: &str = "./loader.toml";

const LIB_TEMPLATE: &str = r"EESchema-LIBRARY Version 2.4
#encoding utf-8
#
#End Doc Library";

const DCM_TEMPLATE: &str = r"EESchema-DOCLIB  Version 2.0
#
#End Doc Library";


fn main() -> std::io::Result<()> {
    // LOAD CONFIG
    let config_path = canonicalize(CONFIG_PATH)?;
    let config_contents = std::fs::read_to_string(config_path)?;
    // config contains information for merging libraries
    let config: Config = toml::from_str(&config_contents)?;
    
    // FIND LIBRARY FILES
    // libs is an iterator through the library files found in the download folder
    let libs = std::fs::read_dir(canonicalize(&config.download_folder)?)?.filter(|entry_res| {
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
    });
    
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
        Err(_) => String::from(LIB_TEMPLATE),
    };
    
    // Attempt to read the dcm file
    let dcm_file_contents = match read_to_string(&dcm_file_path) {
        Ok(contents) => contents,
        Err(_) => String::from(DCM_TEMPLATE),
    };
    Ok(())
}
