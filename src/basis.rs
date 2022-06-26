#[repr(C, align(4096))]
pub struct ListBasisRequest {
    data: [u8; 4096],
}

impl ListBasisRequest {
    pub fn new() -> ListBasisRequest {
        ListBasisRequest { data: [0u8; 4096] }
    }

    pub fn invoke(mut self, connection: u32) -> Result<BasisList, ()> {
        let memory_range = unsafe {
            xous::MemoryRange::new(
                &mut self.data as *mut _ as usize,
                core::mem::size_of::<ListBasisRequest>(),
            )
            .unwrap()
        };

        let result = xous::send_message(
            connection,
            xous::Message::new_lend_mut(
                crate::Opcodes::ListBasisStd as usize,
                memory_range,
                None,
                core::num::NonZeroUsize::new(4096),
            ),
        );

        if let Ok(xous::Result::MemoryReturned(_, _)) = result {
            Ok(BasisList::new(self.data).unwrap())
        } else {
            Err(())
        }
    }
}

#[repr(C, align(4096))]
pub struct BasisList {
    data: [u8; 4096],
}

impl BasisList {
    pub fn new(buffer: [u8; 4096]) -> Option<Self> {
        let version = u32::from_le_bytes(buffer[0..4].try_into().unwrap());
        // println!("Version returned: {}", version);

        if version != 1 {
            return None;
        }

        Some(BasisList { data: buffer })
    }

    pub fn len(&self) -> usize {
        self.data[4] as usize
    }

    pub fn iter(&self) -> BasisListIter {
        BasisListIter::new(self)
    }
}

pub struct BasisListIter<'a> {
    data: &'a [u8],
    index: usize,
    running_offset: usize,
}

impl<'a> BasisListIter<'a> {
    pub fn new(list: &'a BasisList) -> Self {
        let len = list.data[1] as usize;
        BasisListIter {
            data: list.data.as_slice(),
            index: 0,
            // Set the running offset to point at the first entry, which is
            // 4 bytes of version plus one byte of length data plus the
            // length of the lengths.
            running_offset: 4 + 1 + len,
        }
    }
}

impl<'a> Iterator for BasisListIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.data[4] as usize {
            return None;
        }

        let current_value = core::str::from_utf8(
            &self.data[self.running_offset
                ..=self.running_offset + self.data[4 + 1 + self.index] as usize],
        )
        .ok();

        // Skip past the current string in preparation for the next string
        self.running_offset += self.data[4 + 1 + self.index] as usize;
        self.index += 1;

        current_value
    }
}
