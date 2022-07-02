use crate::senres::Senres;

pub struct BasisList {
    data: crate::senres::Stack<4096>,
    count: usize,
}

impl BasisList {
    pub fn new(connection: u32) -> Option<Self> {
        let mut request = crate::senres::Stack::<4096>::new();

        // Request version 1 of the buffer
        {
            let mut writer = request.writer(*b"basQ")?;
            writer.append(1u32);
        }

        request
            .lend_mut(connection, crate::Opcodes::ListBasisStd as usize)
            .unwrap();

        let reader = request.reader(*b"basR").expect("unable to get reader");
        let version: u32 = reader.try_get_from().unwrap();
        if version != 1 {
            return None;
        }

        let count = reader.try_get_from::<u32>().unwrap() as usize;
        Some(BasisList {
            data: request,
            count,
        })
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn iter(&self) -> BasisListIter {
        BasisListIter::new(self)
    }
}

pub struct BasisListIter<'a> {
    index: core::cell::Cell<usize>,
    length: usize,
    reader: crate::senres::Reader<'a, crate::senres::Stack<4096>>,
}

impl<'a> BasisListIter<'a> {
    pub fn new(list: &'a BasisList) -> Self {
        let reader = list.data.reader(*b"basR").unwrap();
        reader.try_get_from::<u32>().unwrap();
        let length = reader.try_get_from::<u32>().unwrap() as usize;
        BasisListIter {
            reader,
            index: core::cell::Cell::new(0),
            length,
        }
    }
}

impl<'a> Iterator for &'a BasisListIter<'_> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        if self.index.get() >= self.length {
            return None;
        }

        self.index.set(self.index.get() + 1);
        self.reader.try_get_ref_from().ok()
    }
}
