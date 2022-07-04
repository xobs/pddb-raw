const SERVER_NAME_KEYS: &str = "_Root key server and update manager_";
const SERVER_NAME_PDDB: &str = "_Plausibly Deniable Database_";

use std::fmt::Write as _;
use std::fs::File;
use std::path::Path;
use std::{io::Read, io::Seek, io::Write};

#[cfg(target_os = "xous")]
use std::os::xous::path::PathExt;

mod services;

mod basis;
mod dict;
mod key;
mod path;
mod senres;

#[repr(usize)]
pub(crate) enum Opcodes {
    IsMounted = 0,
    TryMount = 1,

    WriteKeyFlush = 18,
    KeyDrop = 20,

    ListBasisStd = 26,
    ListDictStd = 28,
    ListKeyStd = 29,

    OpenKeyStd = 30,
    ReadKeyStd = 31,
    WriteKeyStd = 32,

    ListPathStd = 37,
}

fn unlock_db(key: &str) {
    #[repr(C, align(4096))]
    struct UnlockPasswordRequest {
        key: [u8; 4096],
    }

    const BOOT_PASSWORD_OPCODE: usize = 34;
    let root_keys = services::connect(SERVER_NAME_KEYS).unwrap();
    let mut unlock_request = UnlockPasswordRequest { key: [0u8; 4096] };

    for (dest, src) in unlock_request.key.iter_mut().zip(key.as_bytes()) {
        *dest = *src;
    }

    let memory_range = unsafe {
        xous::MemoryRange::new(
            &mut unlock_request as *mut UnlockPasswordRequest as usize,
            core::mem::size_of::<UnlockPasswordRequest>(),
        )
        .unwrap()
    };

    xous::send_message(
        root_keys,
        xous::Message::new_lend(
            BOOT_PASSWORD_OPCODE,
            memory_range,
            None,
            core::num::NonZeroUsize::new(key.len()),
        ),
    )
    .unwrap();
}

struct Pddb {
    cid: u32,
}

impl Pddb {
    pub fn new() -> Self {
        Pddb {
            cid: services::connect(SERVER_NAME_PDDB).unwrap(),
        }
    }

    pub fn is_mounted(&self) -> bool {
        xous::send_message(
            self.cid,
            xous::Message::new_blocking_scalar(crate::Opcodes::IsMounted as usize, 0, 0, 0, 0),
        )
        .map(|v| xous::Result::Scalar1(1) == v)
        .unwrap_or(false)
    }

    pub fn try_mount(&self) -> bool {
        xous::send_message(
            self.cid,
            xous::Message::new_blocking_scalar(crate::Opcodes::TryMount as usize, 0, 0, 0, 0),
        )
        .map(|v| xous::Result::Scalar1(1) == v)
        .unwrap_or(false)
    }

    pub fn list_bases(&self) -> basis::BasisList {
        basis::BasisList::new(self.cid).unwrap()
    }

    // fn list_dictionaries(&self, basis: Option<&str>) -> dict::DictList {
    //     let request = dict::ListDictRequest::new(basis);
    //     request.invoke(self.cid).unwrap()
    // }

    fn list_path(&self, path: &str) -> dict::PathList {
        dict::PathList::new(self.cid, path).unwrap()
    }

    // fn list_keys(&self, basis: Option<&str>, dict: &str) -> key::KeyList {
    //     let request = key::ListKeyRequest::new(basis, dict);
    //     request.invoke(self.cid).unwrap()
    // }
}

fn recursively_list_dirs<P: AsRef<Path>>(root: P) {
    use std::fs;
    let root = root.as_ref();
    println!("Recursively listing \"{}\"", root.display());

    fn visit_dirs(dir: &Path, depth: usize) -> std::io::Result<()> {
        let entries = match fs::read_dir(dir) {
            Ok(o) => o,

            Err(e) => {
                println!("error reading {}: {}", dir.display(), e);
                return Ok(());
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(o) => o,
                Err(e) => {
                    println!("error: {}", e);
                    continue;
                }
            };
            // let mut path = dir.to_owned();
            // path.push(entry.path());
            let path = entry.path();
            print!("|");
            let kind;
            let mut should_recurse = false;

            let is_dir = path.is_dir();
            let is_file = path.is_file();
            #[cfg(target_os = "xous")]
            let is_basis = path.is_basis();
            #[cfg(not(target_os = "xous"))]
            let is_basis = false;

            if is_basis {
                should_recurse = true;
                kind = "[BASIS]";
            } else if is_dir && is_file {
                kind = "[DIR/FILE]";
                should_recurse = true;
            } else if is_dir {
                kind = "[DIR]";
                should_recurse = true;
            } else if is_file {
                kind = "[KEY]";
            } else {
                kind = "[UNKNOWN]";
            }

            for _ in 0..depth * 4 {
                print!(" ");
            }
            print!("{:50}", path.display());
            for _ in 0..(24usize.saturating_sub(depth * 4)) {
                print!(" ");
            }
            println!("{}", kind);
            if should_recurse {
                // let mut path_down = dir.to_owned();
                // path_down.push(&path);
                visit_dirs(&path, depth + 1)?;
            }
            continue;
        }

        Ok(())
    }

    visit_dirs(root, 1).unwrap();
    println!();
}

fn main() {
    println!("PDDB Raw Operations");
    println!("Unlocking DB...");
    unlock_db("a");

    // The PDDB seems to take a long time to start up
    let start_time = std::time::Instant::now();
    let mut try_mount_calls = 0;
    println!("Connecting to PDDB...");
    let pddb = Pddb::new();
    println!(
        "Starting mount (elapsed: {} ms)",
        start_time.elapsed().as_millis()
    );
    loop {
        pddb.try_mount();
        try_mount_calls += 1;
        if pddb.is_mounted() {
            break;
        }
    }
    println!(
        "PDDB mounted with {} try_mount calls after {} ms",
        try_mount_calls,
        start_time.elapsed().as_millis()
    );

    println!("Doing other operations...");
    {
        let list = pddb.list_bases();
        println!("There are {} bases", list.len());
        for entry in &list.iter() {
            println!("Basis: {}", entry);
        }
    }

    {
        println!("Opening file sys.rtc:tz_offset");
        let mut f = File::open("sys.rtc:tz_offset").expect("couldn't open tz_offset file!");
        let mut buf = vec![];
        let bytes_read = f
            .read_to_end(&mut buf)
            .expect("couldn't read contents of file");
        println!("Read {} bytes of data: {:?}", bytes_read, buf);
    }

    println!("Opening file wlan.networks:Renode");
    if let Ok(mut f) = File::open("wlan.networks:Renode") {
        let mut buf = vec![];
        let bytes_read = f
            .read_to_end(&mut buf)
            .expect("couldn't read contents of file");
        println!("Read {} bytes of data: {:?}", bytes_read, buf);
        if let Ok(val) = core::str::from_utf8(&buf) {
            println!("Data as string: [{}]", val);
        }
    }

    {
        for path in [
            "",
            ":",
            "wlan.networks",
            "sys.rtc",
            "fido.cfg",
            "vault.passwords",
        ] {
            println!("Listing path {}", path);
            let entries = pddb.list_path(path);
            for entry in entries.iter() {
                println!("{:?}", entry);
            }
            println!();
        }
    }

    println!("Going to recursively list directories...");
    recursively_list_dirs(Path::new(""));
    recursively_list_dirs(Path::new("::"));
    recursively_list_dirs(Path::new(":"));
    recursively_list_dirs(Path::new(":.System"));
    recursively_list_dirs(Path::new("sys.rtc"));

    // println!("Opening a file...");
    // let mut file = key::Key::open(pddb.cid, None, "wlan.networks", "Renode").unwrap();
    // println!("Reading password...");
    // let mut password = String::new();
    // let len = file
    //     .read_to_string(&mut password)
    //     .expect("Unable to read password");
    // println!("Password is {} bytes long: {}", len, password);
    // println!("Appending {} to the password", password.len());
    // write!(password, "{}", password.len() + 1).unwrap();
    // println!("Writing {} to password field", password);
    // file.rewind().expect("couldn't rewind password file");
    // file.write_all(password.as_bytes())
    //     .expect("unable to update password");

    println!("Done with operations");
}
