use std::fmt::Display;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Serialize, Deserialize};
use serde_json;

/// Load a file containing json encoded data if it exists. If it does not exist return the default
/// for the target type, and generate the file.
pub fn load_json_or_default<T, P>(path: P) -> T
    where T: Serialize + Deserialize + Default,
          P: AsRef<Path> + Display
{
    match File::open(&path).map(|mut r| serde_json::from_reader(&mut r)) {
        // The file existed and we were able to parse the file
        Ok(Ok(data)) => return data,

        // The file existed but was invalid, generate a panic so the user has the change to fix
        // (or remove) the config file.
        Ok(Err(e)) => panic!("Failed to parse `{}`: {}", path, e),

        // File did not exist
        Err(ref e) if e.kind() == ErrorKind::NotFound => {},

        // We were unable to read the file for some other reason
        Err(e) => panic!("Unable to access `{}`: {}", path, e),
    }

    let config: T = Default::default();
    if let Err(e) = File::create(&path).map(|mut w| serde_json::to_writer_pretty(&mut w, &config)) {
        error!(target: "io", "Failed to save default file `{}`: {}", path, e);
    }

    config
}
