const SERVER_NAME_KEYS: &str = "_Root key server and update manager_";
const SERVER_NAME_PDDB: &str = "_Plausibly Deniable Database_";

use std::{io::Read, io::Seek, io::Write};

mod services;

mod basis;
mod dict;
mod key;
mod path;

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

    pub fn list_basis(&self) -> basis::BasisList {
        let request = basis::ListBasisRequest::new();
        request.invoke(self.cid).unwrap()
    }

    fn list_dictionaries(&self, basis: Option<&str>) -> dict::DictList {
        let request = dict::ListDictRequest::new(basis);
        request.invoke(self.cid).unwrap()
    }

    fn list_keys(&self, basis: Option<&str>, dict: &str) -> key::KeyList {
        let request = key::ListKeyRequest::new(basis, dict);
        request.invoke(self.cid).unwrap()
    }
}

fn main() {
    println!("PDDB Raw Operations");
    std::thread::sleep(std::time::Duration::from_millis(300));
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
        let list = pddb.list_basis();
        println!("There are {} bases", list.len());
        for entry in list.iter() {
            println!("Basis: {}", entry);
        }
    }

    {
        let dicts = pddb.list_dictionaries(None);
        println!("There are {} dicts in the union basis", dicts.len());
        for dict in dicts.iter() {
            println!("Dict: {}", dict);

            let keys = pddb.list_keys(None, dict);
            println!("There are {} keys in the {} dict", keys.len(), dict);
            for key in keys.iter() {
                println!("    key: {}", key);
            }
        }
    }

    println!("Opening a file...");
    let mut file = key::Key::open(pddb.cid, None, "wlan.networks", "Renode").unwrap();
    println!("Reading password...");
    let mut password = String::new();
    let len = file
        .read_to_string(&mut password)
        .expect("Unable to read password");
    println!("Password is {} bytes long: {}", len, password);
    println!("Appending {} to the password", password.len());
    password.push_str(&format!("{}", password.len() + 1));
    println!("Writing {} to password field", password);
    file.rewind().expect("couldn't rewind password file");
    file.write_all(password.as_bytes())
        .expect("unable to update password");

    println!("Done with operations");
}
