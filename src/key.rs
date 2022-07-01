#[repr(C, align(4096))]
pub struct ListKeyRequest {
    data: [u8; 4096],
}

/// Return codes for Read/Write API calls to the main server
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PddbRetcode {
    Uninit = 0,
    Ok = 1,
    BasisLost = 2,
    AccessDenied = 3,
    UnexpectedEof = 4,
    InternalError = 5,
    DiskFull = 6,
    Invalid = u8::MAX,
}

impl From<PddbRetcode> for std::io::ErrorKind {
    fn from(other: PddbRetcode) -> Self {
        match other {
            PddbRetcode::BasisLost => std::io::ErrorKind::NotFound,
            PddbRetcode::UnexpectedEof => std::io::ErrorKind::UnexpectedEof,
            PddbRetcode::DiskFull => std::io::ErrorKind::OutOfMemory,
            _ => std::io::ErrorKind::Other,
        }
    }
}

impl From<usize> for PddbRetcode {
    fn from(val: usize) -> Self {
        use PddbRetcode::*;
        match val {
            0 => Uninit,
            1 => Ok,
            2 => BasisLost,
            3 => AccessDenied,
            4 => UnexpectedEof,
            5 => InternalError,
            6 => DiskFull,
            _ => Invalid,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
struct ApiToken([u32; 3]);

/// PddbBuf is a C-representation of a page of memory that's used
/// to shuttle data for streaming channels. It must be exactly one
/// page in size, with some overhead specific to the PDDB book-keeping
/// at the top, and the remainder available for shuttling data.
#[repr(C, align(4096))]
#[derive(Debug)]
struct PddbBuf {
    /// api token for the given buffer
    token: ApiToken,
    /// a field reserved for the return code
    retcode: PddbRetcode,
    reserved: u8,
    /// length of the data field
    len: u16,
    /// point in the key stream. 64-bit for future-compatibility; but, can't be larger than 32 bits on a 32-bit target.
    position: u64,
    data: [u8; 4072],
}

#[allow(dead_code)]
const fn _assert_pddbbuf_is_4096_bytes() {
    unsafe {
        core::mem::transmute::<_, PddbBuf>([0u8; 4096]);
    }
}

#[repr(C, align(4096))]
pub struct OpenKeyRequest {
    data: [u8; 4096],
}

pub struct Key {
    fd: ApiToken,
    connection: u32,
    offset: u64,
    len: u64,
}

impl Key {
    pub fn open(
        connection: xous::CID,
        basis: Option<&str>,
        dict: &str,
        key: &str,
    ) -> Result<Key, ()> {
        let create_key = false;
        let create_dict = false;
        let cb_sid: Option<xous::SID> = None;
        let mut req = OpenKeyRequest { data: [0u8; 4096] };

        let mut read_offset = 0;

        let request_version: u32 = 1usize.try_into().unwrap();
        // Version number of the request
        for (src, dest) in request_version
            .to_le_bytes()
            .iter()
            .zip(req.data[0..4].iter_mut())
        {
            *dest = *src;
        }
        read_offset += 4;

        // If there's a name, add that
        if let Some(basis) = basis {
            let name_length = basis.len() as u32;
            for (src, dest) in name_length
                .to_le_bytes()
                .iter()
                .zip(req.data[read_offset..read_offset + 4].iter_mut())
            {
                *dest = *src;
            }
            read_offset += 4;
            // Copy the name bytes
            for (src, dest) in basis
                .as_bytes()
                .iter()
                .zip(req.data[read_offset..].iter_mut())
            {
                *dest = *src;
            }
            read_offset += basis.len();
        }
        // Otherwise, zero out the "name" field
        else {
            // Write "0" for the length
            for dest in req.data[read_offset..read_offset + 4].iter_mut() {
                *dest = 0;
            }
            read_offset += 4;
        }

        // Copy the dict name
        let name_length = dict.len() as u32;
        for (src, dest) in name_length
            .to_le_bytes()
            .iter()
            .zip(req.data[read_offset..read_offset + 4].iter_mut())
        {
            *dest = *src;
        }
        read_offset += 4;
        // Copy the name bytes
        for (src, dest) in dict
            .as_bytes()
            .iter()
            .zip(req.data[read_offset..].iter_mut())
        {
            *dest = *src;
        }
        read_offset += dict.len();

        // Copy the key name
        let name_length = key.len() as u32;
        for (src, dest) in name_length
            .to_le_bytes()
            .iter()
            .zip(req.data[read_offset..read_offset + 4].iter_mut())
        {
            *dest = *src;
        }
        read_offset += 4;
        // Copy the key name bytes
        for (src, dest) in key
            .as_bytes()
            .iter()
            .zip(req.data[read_offset..].iter_mut())
        {
            *dest = *src;
        }
        read_offset += key.len();

        req.data[read_offset] = if create_dict { 1 } else { 0 };
        read_offset += 1;

        req.data[read_offset] = if create_key { 1 } else { 0 };
        read_offset += 1;

        let alloc_hint = 0u64;
        for (src, dest) in alloc_hint.to_le_bytes().iter().zip(req.data[read_offset..read_offset+8].iter_mut()) {
            *dest = *src;
        }
        read_offset += 8;
    

        if let Some(cb_sid) = cb_sid {
            req.data[read_offset] = 1;
            read_offset += 1;

            for word in cb_sid.to_array().iter() {
                for (src, dest) in word
                    .to_le_bytes()
                    .iter()
                    .zip(req.data[read_offset..read_offset + 4].iter_mut())
                {
                    *dest = *src;
                }
                read_offset += 4;
            }
        } else {
            req.data[read_offset] = 0;
            // read_offset += 1;
            // read_offset += 16;
        }

        let memory_range = unsafe {
            xous::MemoryRange::new(
                &mut req.data as *mut _ as usize,
                core::mem::size_of::<OpenKeyRequest>(),
            )
            .unwrap()
        };

        let result = xous::send_message(
            connection,
            xous::Message::new_lend_mut(
                crate::Opcodes::OpenKeyStd as usize,
                memory_range,
                None,
                core::num::NonZeroUsize::new(4096),
            ),
        );

        if let Ok(xous::Result::MemoryReturned(_, _)) = result {
            let result = u32::from_le_bytes(req.data[0..4].try_into().unwrap()) as usize;
            if result == 0 {
                let mut fd = ApiToken::default();
                for (chunk, word) in req.data[4..].chunks(4).zip(fd.0.iter_mut()) {
                    *word = u32::from_le_bytes(chunk.try_into().unwrap());
                }
                let len = u64::from_le_bytes(req.data[16..24].try_into().unwrap());
                Ok(Key {
                    fd,
                    connection,
                    len,
                    offset: 0,
                })
            } else {
                println!("Error code: {:?}", result);
                Err(())
            }
        } else {
            println!("Unexpected return: {:?}", result);
            Err(())
        }
    }
}

impl std::io::Read for Key {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut pddb_buffer = PddbBuf {
            /// api token for the given buffer
            token: self.fd,
            /// a field reserved for the return code
            retcode: PddbRetcode::Uninit,
            reserved: 0,
            /// length of the data field
            len: buf.len().try_into().unwrap_or(u16::MAX).min(4072),
            /// point in the key stream. 64-bit for future-compatibility; but, can't be larger than 32 bits on a 32-bit target.
            position: self.offset,
            data: [0u8; 4072],
        };

        let memory_range = unsafe {
            xous::MemoryRange::new(
                &mut pddb_buffer as *mut _ as usize,
                core::mem::size_of::<PddbBuf>(),
            )
            .unwrap()
        };

        let result = xous::send_message(
            self.connection,
            xous::Message::new_lend_mut(
                crate::Opcodes::ReadKeyStd as usize,
                memory_range,
                None,
                core::num::NonZeroUsize::new(4096),
            ),
        );

        if let Ok(xous::Result::MemoryReturned(_, _)) = result {
            if pddb_buffer.retcode == PddbRetcode::Ok {
                let contents = &pddb_buffer.data[0..pddb_buffer.len as usize];
                for (src, dest) in contents.iter().zip(buf.iter_mut()) {
                    *dest = *src;
                }
                self.offset += pddb_buffer.len as u64;
                Ok(pddb_buffer.len.into())
            } else {
                println!("Pddb error: {:?}", pddb_buffer);
                Err(std::io::Error::new(pddb_buffer.retcode.into(), "key error"))
            }
        } else {
            println!("Unexpected return: {:?}", result);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "key error"))
        }
    }
}

impl std::io::Write for Key {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut pddb_buffer = PddbBuf {
            /// api token for the given buffer
            token: self.fd,
            /// a field reserved for the return code
            retcode: PddbRetcode::Uninit,
            reserved: 0,
            /// length of the data field
            len: buf.len().try_into().unwrap_or(u16::MAX).min(4072),
            /// point in the key stream. 64-bit for future-compatibility; but, can't be larger than 32 bits on a 32-bit target.
            position: self.offset,
            data: [0u8; 4072],
        };

        // Copy the data to the buffer for writing
        for (src, dest) in buf.iter().zip(pddb_buffer.data.iter_mut()) {
            *dest = *src;
        }

        let memory_range = unsafe {
            xous::MemoryRange::new(
                &mut pddb_buffer as *mut _ as usize,
                core::mem::size_of::<PddbBuf>(),
            )
            .unwrap()
        };

        let result = xous::send_message(
            self.connection,
            xous::Message::new_lend_mut(
                crate::Opcodes::WriteKeyStd as usize,
                memory_range,
                None,
                core::num::NonZeroUsize::new(4096),
            ),
        );

        if let Ok(xous::Result::MemoryReturned(_, _)) = result {
            if pddb_buffer.retcode == PddbRetcode::Ok {
                self.offset += pddb_buffer.len as u64;
                // If we've written past the end of the file, update the file length
                if self.offset > self.len {
                    self.len = self.offset;
                }
                Ok(pddb_buffer.len.into())
            } else {
                println!("Pddb error: {:?}", pddb_buffer);
                Err(std::io::Error::new(pddb_buffer.retcode.into(), "key error"))
            }
        } else {
            println!("Unexpected return: {:?}", result);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "key error"))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = xous::send_message(
            self.connection,
            xous::Message::new_blocking_scalar(
                crate::Opcodes::WriteKeyFlush as usize,
                self.fd.0[0].try_into().unwrap(),
                self.fd.0[1].try_into().unwrap(),
                self.fd.0[2].try_into().unwrap(),
                0,
            ),
        );

        if let Ok(xous::Result::Scalar1(val)) = result {
            if val == PddbRetcode::Ok as _ {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    PddbRetcode::from(val).into(),
                    "flush error",
                ))
            }
        } else {
            panic!("Unexpected return from WriteKeyFlush: {:?}", result);
        }
    }
}

impl std::io::Seek for Key {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        fn seek_from_point(this: &mut Key, point: u64, by: i64) -> std::io::Result<u64> {
            let by64 = by as u64;
            // Note that it's possible to seek past the end of a key, and in this case
            // the `offset` will be greater than the `len`. This is fine, and `len` will
            // be updated as soon as `write()` is called.
            if by < 0 {
                this.offset = point.checked_sub(by64).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "cannot seek before 0")
                })?;
            } else {
                this.offset = point.checked_add(by64).ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "seek overflowed")
                })?;
            }
            Ok(this.offset)
        }

        use std::io::SeekFrom;
        match pos {
            SeekFrom::Start(offset) => seek_from_point(self, 0, offset as i64),
            SeekFrom::Current(by) => seek_from_point(self, self.offset, by),
            SeekFrom::End(by) => seek_from_point(self, self.len, by),
        }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        let result = xous::send_message(
            self.connection,
            xous::Message::new_blocking_scalar(
                crate::Opcodes::KeyDrop as usize,
                self.fd.0[0].try_into().unwrap(),
                self.fd.0[1].try_into().unwrap(),
                self.fd.0[2].try_into().unwrap(),
                0,
            ),
        );

        if let Ok(xous::Result::Scalar1(1)) = result {
        } else {
            panic!("Unexpected return from KeyDrop: {:?}", result);
        }
    }
}

impl ListKeyRequest {
    pub fn new(basis: Option<&str>, dict: &str) -> ListKeyRequest {
        let mut this = ListKeyRequest { data: [0u8; 4096] };
        Self::set_version(&mut this, 1);
        Self::set_basis(&mut this, basis);

        // This call must come after `set_basis()`.
        Self::set_dict(&mut this, dict);

        this
    }

    pub fn invoke(mut self, connection: u32) -> Result<KeyList, ()> {
        let memory_range = unsafe {
            xous::MemoryRange::new(
                &mut self.data as *mut _ as usize,
                core::mem::size_of::<ListKeyRequest>(),
            )
            .unwrap()
        };

        let result = xous::send_message(
            connection,
            xous::Message::new_lend_mut(
                crate::Opcodes::ListKeyStd as usize,
                memory_range,
                None,
                core::num::NonZeroUsize::new(4096),
            ),
        );

        if let Ok(xous::Result::MemoryReturned(_, _)) = result {
            Ok(KeyList::new(self.data).unwrap())
        } else {
            Err(())
        }
    }

    fn set_version(&mut self, request_version: usize) {
        let request_version: u32 = request_version.try_into().unwrap();
        // Version number of the request
        for (src, dest) in request_version
            .to_le_bytes()
            .iter()
            .zip(self.data[0..4].iter_mut())
        {
            *dest = *src;
        }
    }

    fn set_basis(&mut self, basis: Option<&str>) {
        // If there's a name, add that
        if let Some(basis) = basis {
            let name_length = basis.len() as u32;
            for (src, dest) in name_length
                .to_le_bytes()
                .iter()
                .zip(self.data[4..8].iter_mut())
            {
                *dest = *src;
            }
            // Copy the name bytes
            for (src, dest) in basis.as_bytes().iter().zip(self.data[4..].iter_mut()) {
                *dest = *src;
            }
        }
        // Otherwise, zero out the "name" field
        else {
            // Write "0" for the length
            for dest in self.data[4..8].iter_mut() {
                *dest = 0;
            }
        }
    }

    fn set_dict(&mut self, dict: &str) {
        let offset = 8 + u32::from_le_bytes(self.data[4..8].try_into().unwrap()) as usize;

        let name_length = dict.len() as u32;
        for (src, dest) in name_length
            .to_le_bytes()
            .iter()
            .zip(self.data[offset..offset + 4].iter_mut())
        {
            *dest = *src;
        }

        let offset = offset + 4;

        // Copy the name bytes
        for (src, dest) in dict.as_bytes().iter().zip(self.data[offset..].iter_mut()) {
            *dest = *src;
        }
    }
}

pub struct KeyList {
    data: [u8; 4096],
}

impl KeyList {
    pub fn new(buffer: [u8; 4096]) -> Option<Self> {
        let version = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        if version != 1 {
            return None;
        }
        Some(KeyList { data: buffer })
    }

    pub fn len(&self) -> usize {
        self.data[4] as usize
    }

    pub fn iter(&self) -> KeyListIter {
        KeyListIter::new(self)
    }
}

pub struct KeyListIter<'a> {
    data: &'a [u8],
    index: usize,
    running_offset: usize,
}

impl<'a> KeyListIter<'a> {
    pub fn new(list: &'a KeyList) -> Self {
        let len = list.data[4] as usize;
        KeyListIter {
            data: list.data.as_slice(),
            index: 0,
            // Set the running offset to point at the first entry, which is
            // 4 bytes of version plus one byte of length data plus the
            // length of the lengths.
            running_offset: 4 + 1 + len,
        }
    }
}

impl<'a> Iterator for KeyListIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        let current_value = core::str::from_utf8(
            &self.data
                [self.running_offset..self.running_offset + self.data[4 + 1 + self.index] as usize],
        )
        .ok();

        if self.index >= self.data[4] as usize {
            return None;
        }

        // Skip past the current string in preparation for the next string
        self.running_offset += self.data[4 + 1 + self.index] as usize;
        self.index += 1;

        current_value
    }
}
