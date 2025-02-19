mod my_program;

use crate::my_program::my_program;
use std::error::Error;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

use valence_program_manager::program_config::ProgramConfig;

fn main() -> Result<(), Box<dyn Error>> {
    let program = my_program();

    // Print instantiated program to file
    save_config_to_json_file(program)?;

    Ok(())
}

fn save_config_to_json_file(program_config: ProgramConfig) -> Result<(), Box<dyn Error>> {
    let path_name = "deployment/output_program";
    let path = Path::new(path_name);

    if !path.exists() {
        fs::create_dir_all(path_name)?;
    }

    // Construct the full file path
    let file_path = path.join("program.json");

    // Create and write to the file
    let file = fs::File::create(file_path.clone())?;
    let mut writer = BufWriter::new(file);

    // Serialize the data to a string
    serde_json::to_writer(&mut writer, &program_config)?;
    writer.flush()?;

    println!("Program config was built successfully.");

    Ok(())
}
