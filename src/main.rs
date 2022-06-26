const SERVER_NAME_KEYS: &str = "_Root key server and update manager_";
const SERVER_NAME_PDDB: &str = "_Plausibly Deniable Database_";

mod services;

mod basis;
mod dict;

#[repr(usize)]
pub(crate) enum Opcodes {
    ListBasisStd = 26,
    ListDictStd = 28,
}

#[repr(C, align(4096))]
struct UnlockPasswordRequest {
    key: [u8; 4096],
}

fn unlock_db(key: &str) {
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
    // const LIST_BASIS_OPCODE: usize = 2;
    pub fn new() -> Self {
        Pddb {
            cid: services::connect(SERVER_NAME_PDDB).unwrap(),
        }
    }

    pub fn list_basis(&self) -> basis::BasisList {
        let request = basis::ListBasisRequest::new();
        request.invoke(self.cid).unwrap()
    }

    fn list_dictionaries(&self, basis: Option<&str>) -> dict::DictList {
        let request = dict::ListDictRequest::new(basis);

        request.invoke(self.cid).unwrap()

    }
}

fn main() {
    println!("PDDB Raw Operations");
    std::thread::sleep(std::time::Duration::from_millis(300));
    println!("Unlocking DB...");
    unlock_db("a");

    // The PDDB seems to take a long time to start up
    std::thread::sleep(std::time::Duration::from_secs(4));
    println!("Doing other operations...");
    let pddb = Pddb::new();
    {
        let list = pddb.list_basis();
        println!("There are {} bases", list.len());
        for entry in list.iter() {
            println!("Basis: {}", entry);
        }
    }

    {
        let list = pddb.list_dictionaries(None);
        println!("There are {} dicts in the union basis", list.len());
        for entry in list.iter() {
            println!("Dict: {}", entry);
        }
    }
    println!("Done with operations");
}
