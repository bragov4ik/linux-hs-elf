use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use clap::Parser;
use object::{StringTable, Endianness};
use object::elf::{FileHeader64, DT_NEEDED, DT_STRTAB, DT_STRSZ};
use object::read::elf::{FileHeader, Dyn};
use tracing::{warn, debug};

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
    NotElf,
}

fn extract_libs<H>(bin_data: &[u8], endian: Endianness, header: &H) -> Result<Vec<String>, HandleError>
where
    H: FileHeader<Endian = Endianness>,
{
    let s = header.sections(
        endian, bin_data
    )
        .map_err(HandleError::ObjectReadError)?;
    let dyn_sec = s.dynamic(
        endian, bin_data
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
                let offs = dyn_element.d_val(endian).into(); 
                debug!("Found required dyn library at offset {}", offs);
                libs_offs.push(offs);
            },
            Some(DT_STRTAB) => {
                dt_strtab = dyn_element.d_val(endian).into();
            },
            Some(DT_STRSZ) => {
                dt_strsz = dyn_element.d_val(endian).into();
            }
            _ => warn!("Dynamic element's tag {} does not fit into u32", dyn_element.d_tag(endian).into()),
        }
    }
    let libs_offs = libs_offs.iter()
        .map(|n| u32::try_from(*n).ok());
    let str_table = StringTable::new(
        bin_data, dt_strtab, dt_strtab + dt_strsz
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
            warn!("Couldn't get lib name by offset {}, strtab {}", offs, dt_strtab);
            continue;
        }
    }
    Ok(libs)
}

fn get_needed_libs<P>(path: P) -> Result<Vec<String>, HandleError>
where
    P: AsRef<std::path::Path> 
{
    let bin_data = fs::read(path)
        .map_err(HandleError::IoError)?;
    
    let kind = match object::FileKind::parse(bin_data.as_slice()) {
        Ok(k) => k,
        Err(e) => {
            warn!("Could not parse file");
            return Err(HandleError::ObjectReadError(e));
        },
    };

    match kind {
        object::FileKind::Elf32 => {
            debug!("Parsing elf32 file");
            let elf_header = FileHeader64::<object::Endianness>::parse(&*bin_data)
                .map_err(HandleError::ObjectReadError)?;
            let endian = elf_header.endian().unwrap();
            extract_libs(bin_data.as_slice(), endian, elf_header)
        },
        object::FileKind::Elf64 => {
            debug!("Parsing elf64 file");
            let elf_header = FileHeader64::<object::Endianness>::parse(&*bin_data)
                .map_err(HandleError::ObjectReadError)?;
            let endian = elf_header.endian().unwrap();
            extract_libs(bin_data.as_slice(), endian, elf_header)
        },
        _ => Err(HandleError::NotElf)
    }
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
        debug!("Handling file {}", filename.to_str().unwrap());
        match get_needed_libs(dir_entry.path()) {
            Ok(libs) => {
                for lib in libs {
                    lib_map.entry(lib).or_default().push(
                        filename.to_str().unwrap().to_string()
                    );
                }
            },
            Err(e) => warn!(
                "Couldn't handle {}: {:?}", dir_entry.file_name().to_str().unwrap(), e
            ),
        }
    }
    let mut lib_list: Vec<(String, Vec<String>)> = lib_map.into_iter().collect();
    lib_list.sort_by_key(|p| p.1.len());
    for (lib, exes) in lib_list {
        println!("{} ({} exes)", lib, exes.len());
        for exe in exes {
            println!("\t<= {}", exe);
        }
        println!()
    }
}
