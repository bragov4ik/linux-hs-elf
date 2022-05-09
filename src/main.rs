use std::path::PathBuf;
use std::fs;
use clap::Parser;
use object::Object;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long, parse(from_os_str), value_name = "executables-dir", default_value = "/")]
    executables_dir: PathBuf,
}

#[derive(Debug)]
enum HandleError {
    IoError(std::io::Error),
    ObjectReadError(object::read::Error),
}

fn handle_path<P>(path: P) -> Result<(), HandleError>
where
    P: AsRef<std::path::Path> 
{
    let filename = path.as_ref()
        .file_name()
        .unwrap_or_default()
        .to_owned();
    let bin_data = fs::read(path)
        .map_err(HandleError::IoError)?;
    let obj_file = object::File::parse(&*bin_data)
        .map_err(HandleError::ObjectReadError)?;
    println!("File {:#?} is {:#?}", filename, obj_file.kind());
    return Ok(());
}

fn main() {
    let args = Args::parse();
    let bin_paths = fs::read_dir(args.executables_dir).expect("Could not list binaries");
    for dir_entry in bin_paths {
        let dir_entry = match dir_entry {
            Ok(p) => p,
            Err(e) => {
                println!("Error getting next path: {:?}", e);
                continue;
            },
        };
        if let Err(e) = handle_path(dir_entry.path()) {
            println!("Error handling {}: {:?}", dir_entry.file_name().to_str().unwrap_or_default(), e);
        }
    }
}
