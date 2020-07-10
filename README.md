# KiCAD CSE LibLoader

This is a program written in rust that takes a set of components downloaded from Component Search Engine and merges them with some main library. The main library name and location as well as the download folder is set in the config file.

## Using the program

Clone this repository and build the program using Cargo (cargo build --release).

Take the executable from target/release and place it anywhere you want then copy loader.toml from this folder to the folder that the executable in.

Modify loader.toml to your liking.

Now you can run the executable whenever you want to update your main lobby.