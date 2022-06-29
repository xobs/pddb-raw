#[repr(C, align(4096))]
pub struct ListDictRequest {
    data: [u8; 4096],
}

impl ListDictRequest {
    pub fn new(basis: Option<&str>) -> ListDictRequest {
        let mut this = ListDictRequest { data: [0u8; 4096] };
        Self::set_version(&mut this, 1);
        Self::set_basis(&mut this, basis);

        this
    }

    pub fn invoke(mut self, connection: u32) -> Result<DictList, ()> {
        let memory_range = unsafe {
            xous::MemoryRange::new(
                &mut self.data as *mut _ as usize,
                core::mem::size_of::<ListDictRequest>(),
            )
            .unwrap()
        };

        let result = xous::send_message(
            connection,
            xous::Message::new_lend_mut(
                crate::Opcodes::ListDictStd as usize,
                memory_range,
                None,
                core::num::NonZeroUsize::new(4096),
            ),
        );

        if let Ok(xous::Result::MemoryReturned(_, _)) = result {
            Ok(DictList::new(self.data).unwrap())
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

    pub fn set_basis(&mut self, basis: Option<&str>) {
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
            for (src, dest) in basis.as_bytes().iter().zip(self.data[8..].iter_mut()) {
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
}

pub struct DictList {
    data: [u8; 4096],
}

impl DictList {
    pub fn new(buffer: [u8; 4096]) -> Option<Self> {
        let version = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        if version != 1 {
            return None;
        }
        Some(DictList { data: buffer })
    }

    pub fn len(&self) -> usize {
        self.data[4] as usize
    }

    pub fn iter(&self) -> DictListIter {
        DictListIter::new(self)
    }
}

pub struct DictListIter<'a> {
    data: &'a [u8],
    index: usize,
    running_offset: usize,
}

impl<'a> DictListIter<'a> {
    pub fn new(list: &'a DictList) -> Self {
        let len = list.data[4] as usize;
        DictListIter {
            data: list.data.as_slice(),
            index: 0,
            // Set the running offset to point at the first entry, which is
            // 4 bytes of version plus one byte of length data plus the
            // length of the lengths.
            running_offset: 4 + 1 + len,
        }
    }
}

impl<'a> Iterator for DictListIter<'a> {
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
