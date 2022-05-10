use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use clap::Parser;
use object::StringTable;
use object::elf::{FileHeader64, DT_NEEDED, DT_STRTAB, DT_STRSZ};
use object::read::elf::{FileHeader, Dyn};
use tracing::warn;

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
    NoDynamic,
}

fn get_needed_libs<P>(path: P) -> Result<Vec<String>, HandleError>
where
    P: AsRef<std::path::Path> 
{
    let bin_data = fs::read(path)
        .map_err(HandleError::IoError)?;
    let elf_header = FileHeader64::<object::Endianness>::parse(&*bin_data)
        .map_err(HandleError::ObjectReadError)?;
    let endian = elf_header.endian().unwrap();
    let s = elf_header.sections(
        endian, bin_data.as_slice()
    )
        .map_err(HandleError::ObjectReadError)?;
    let dyn_sec = s.dynamic(
        endian, bin_data.as_slice()
    )
        .map_err(HandleError::ObjectReadError)?
        .ok_or(HandleError::NoDynamic)?;
    let mut libs_offs: Vec<u64> = vec![];
    let mut dt_strtab: u64 = 0;
    let mut dt_strsz: u64 = 0;
    for dyn_element in dyn_sec.0 {
        let tag32 = dyn_element.tag32(endian);
        match tag32 {
            Some(DT_NEEDED) => {
                libs_offs.push(dyn_element.d_val(endian).into());
            },
            Some(DT_STRTAB) => {
                dt_strtab = dyn_element.d_val(endian).into();
            },
            Some(DT_STRSZ) => {
                dt_strsz = dyn_element.d_val(endian).into();
            }
            _ => (),
        }
    }
    let libs_offs = libs_offs.iter()
        .map(|n| u32::try_from(*n).ok());
    let str_table = StringTable::new(
        bin_data.as_slice(), dt_strtab, dt_strtab + dt_strsz
    );
    let mut libs: Vec<String> = vec![];
    for offs in libs_offs {
        let offs = if let Some(offs) = offs {
            offs
        }
        else {
            warn!("Couldn't convert offset to u32");
            continue;
        };
        let name = str_table.get(offs)
            .map(String::from_utf8_lossy);
        if let Ok(name) = name {
            libs.push(name.to_string());
        }
        else {
            warn!("Couldn't get lib name by offset");
            continue;
        }
    }
    return Ok(libs);
}

fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let bin_paths = fs::read_dir(args.executables_dir).expect("Could not list binaries");
    let mut lib_map: HashMap<String, Vec<String>> = HashMap::new();
    for dir_entry in bin_paths {
        let dir_entry = match dir_entry {
            Ok(p) => p,
            Err(e) => {
                warn!("Couldn't get next path: {:?}", e);
                continue;
            },
        };
        let filename = dir_entry.file_name();
        match get_needed_libs(dir_entry.path()) {
            Ok(libs) => {
                for lib in libs {
                    lib_map.entry(lib).or_default().push(
                        filename.to_str().unwrap().to_string()
                    );
                }
            },
            Err(e) => warn!(
                "Couldn't handle {}: {:?}", dir_entry.file_name().to_str().unwrap_or_default(), e
            ),
        }
    }
    for (lib, exes) in lib_map {
        println!("{} ({} exes)", lib, exes.len());
        for exe in exes {
            println!("\t<= {}", exe);
        }
        println!()
    }
}
