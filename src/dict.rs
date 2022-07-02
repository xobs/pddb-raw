use crate::senres::Senres;

#[derive(Debug)]
pub enum EntryKind {
    Basis = 0,
    Dict = 1,
    Key = 2,
}

#[derive(Debug)]
pub struct Entry {
    name: String,
    kind: EntryKind,
}
pub struct PathList {
    entries: Vec<Entry>,
}

impl PathList {
    pub fn new(connection: u32, path: &str) -> Option<Self> {
        let mut request = crate::senres::Stack::<4096>::new();

        {
            let mut writer = request.writer(*b"PthQ")?;
            // Request version 1 of the buffer
            writer.append(1u32);
            writer.append(path);
        }

        request
            .lend_mut(connection, crate::Opcodes::ListPathStd as usize)
            .unwrap();

        let reader = request.reader(*b"PthR").expect("unable to get reader");
        if let Ok(1u32) = reader.try_get_from() {
        } else {
            panic!("Unexpected value");
            return None;
        }

        let mut entries = vec![];
        let count = reader.try_get_from::<u32>().unwrap() as usize;
        for _ in 0..count {
            let name = reader.try_get_ref_from::<str>().unwrap().to_owned();
            let kind = match reader.try_get_from::<u8>() {
                Ok(0) => EntryKind::Basis,
                Ok(1) => EntryKind::Dict,
                Ok(2) => EntryKind::Key,
                v => panic!("unexpected entrykind {:?}", v),//return None,
            };
            entries.push(Entry { name, kind });
        }
        Some(PathList { entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }
}