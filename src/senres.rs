// #![allow(unused)]

/// Senres V1 always begins with the number 0x344cb6ca to indicate it's valid.
/// This number will change on subsequent versions.
const SENRES_V1_MAGIC: u32 = 0x344cb6ca;

use core::cell::Cell;
/// A struct to send and receive data
#[repr(C, align(4096))]
#[derive(Debug)]
pub struct SenresStack<const N: usize = 4096> {
    data: [u8; N],
}

pub struct SenresMessage<'a, const N: usize = 4096> {
    data: &'a mut [u8],
}

pub trait Senres {
    fn as_slice(&self) -> &[u8];
    fn as_mut_slice(&mut self) -> &mut [u8];
}

pub struct SenresWriter<'a, Backing: Senres, const N: usize> {
    backing: &'a mut Backing,
    offset: usize,
    valid: usize,
}

pub struct SenresReader<'a, Backing: Senres, const N: usize> {
    backing: &'a Backing,
    offset: Cell<usize>,
    valid: usize,
}

enum InvokeType {
    LendMut = 1,
    Lend = 2,
    // Move = 3,
    // Scalar = 4,
    // BlockingScalar = 5,
}

enum Syscall {
    SendMessage = 16,
}

enum SyscallResult {
    MemoryReturned = 18,
}

pub trait SenSer<Backing: Senres, const N: usize> {
    fn append_to(&self, senres: &mut SenresWriter<Backing, N>);
}

pub trait RecDes<Backing: Senres, const N: usize> {
    fn try_get_from(senres: &SenresReader<Backing, N>) -> Result<Self, ()>
    where
        Self: std::marker::Sized;
}

pub trait RecDesRef<'a, Backing: Senres, const N: usize> {
    fn try_get_ref_from(senres: &'a SenresReader<Backing, N>) -> Result<&'a Self, ()>;
}

impl<const N: usize> SenresStack<N> {
    /// Ensure that `N` is a multiple of 4096. This constant should
    /// be evaluated in the constructor function.
    const CHECK_ALIGNED: () = if N & 4095 != 0 {
        panic!("Senres size must be a multiple of 4096")
    };

    pub const fn new() -> Self {
        // Ensure the `N` that was specified is a multiple of 4096
        #[allow(clippy::no_effect, clippy::let_unit_value)]
        let _ = Self::CHECK_ALIGNED;
        SenresStack { data: [0u8; N] }
    }

    pub fn writer(&mut self) -> SenresWriter<Self, N> {
        let mut writer = SenresWriter {
            backing: self,
            offset: 0,
            valid: 0,
        };
        writer.append(SENRES_V1_MAGIC);
        writer
    }

    pub fn reader(&self) -> Option<SenresReader<Self, N>> {
        let reader = SenresReader {
            backing: self,
            offset: Cell::new(0),
            valid: 0,
        };
        if let Ok(SENRES_V1_MAGIC) = reader.try_get_from() {
            Some(reader)
        } else {
            None
        }
    }
}

impl<const N: usize> Senres for SenresStack<N> {
    fn as_mut_slice(&mut self) -> &mut [u8] {
        self.data.as_mut_slice()
    }
    fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }
}

impl<'a, Backing: Senres, const N: usize> SenresWriter<'a, Backing, N> {
    pub fn append<T: SenSer<Backing, N>>(&mut self, other: T) {
        other.append_to(self);
    }

    #[cfg(not(target_os = "xous"))]
    pub fn lend(&self, connection: u32, opcode: usize) -> Result<(), ()> {
        Ok(())
    }

    #[cfg(not(target_os = "xous"))]
    pub fn lend_mut(mut self, connection: u32, opcode: usize) -> Result<Self, ()> {
        // let (offset, valid) = self.invoke(connection, opcode, InvokeType::LendMut)?;
        Ok(self)
    }

    #[cfg(target_os = "xous")]
    pub fn lend(&self, connection: u32, opcode: usize) -> Result<(), ()> {
        let mut a0 = Syscall::SendMessage as usize;
        let mut a1: usize = connection.try_into().unwrap();
        let mut a2 = opcode;
        let mut a3 = InvokeType::Lend as usize;
        let mut a4 = &self.backing.as_slice().as_ptr() as *const _ as usize;
        let mut a5 = N;
        let mut a6 = 0;
        let mut a7 = self.offset;

        unsafe {
            core::arch::asm!(
                "ecall",
                inlateout("a0") a0,
                inlateout("a1") a1,
                inlateout("a2") a2,
                inlateout("a3") a3,
                inlateout("a4") a4,
                inlateout("a5") a5,
                inlateout("a6") a6,
                inlateout("a7") a7,
            )
        };

        let result = a0;
        if result == SyscallResult::MemoryReturned as usize {
            Ok(())
        } else {
            Err(())
        }
    }

    #[cfg(target_os = "xous")]
    pub fn lend_mut(mut self, connection: u32, opcode: usize) -> Result<Self, ()> {
        let mut a0 = Syscall::SendMessage as usize;
        let mut a1: usize = connection.try_into().unwrap();
        let mut a2 = opcode;
        let mut a3 = InvokeType::LendMut as usize;
        let mut a4 = &mut self.backing.as_mut_slice().as_ptr() as *mut _ as usize;
        let mut a5 = N;
        let mut a6 = 0;
        let mut a7 = self.offset;

        unsafe {
            core::arch::asm!(
                "ecall",
                inlateout("a0") a0,
                inlateout("a1") a1,
                inlateout("a2") a2,
                inlateout("a3") a3,
                inlateout("a4") a4,
                inlateout("a5") a5,
                inlateout("a6") a6,
                inlateout("a7") a7,
            )
        };

        let result = a0;
        let offset = a1;
        let valid = a2;

        if result == SyscallResult::MemoryReturned as usize {
            self.offset = offset;
            self.valid = valid;
            Ok(self)
        } else {
            Err(())
        }
    }
}

impl<'a, Backing: Senres, const N: usize> SenresReader<'a, Backing, N> {
    pub fn try_get_from<T: RecDes<Backing, N>>(&self) -> Result<T, ()> {
        T::try_get_from(self)
    }

    pub fn try_get_ref_from<T: RecDesRef<'a, Backing, N> + ?Sized>(&'a self) -> Result<&'a T, ()> {
        T::try_get_ref_from(self)
    }
}

macro_rules! primitive_impl {
    ($SelfT:ty) => {
        impl<Backing: Senres, const N: usize> SenSer<Backing, N> for $SelfT {
            fn append_to(&self, senres: &mut SenresWriter<Backing, N>) {
                for (src, dest) in self
                    .to_le_bytes()
                    .iter()
                    .zip(senres.backing.as_mut_slice()[senres.offset..].iter_mut())
                {
                    *dest = *src;
                    senres.offset += 1;
                }
            }
        }

        impl<Backing: Senres, const N: usize> RecDes<Backing, N> for $SelfT {
            fn try_get_from(senres: &SenresReader<Backing, N>) -> Result<Self, ()> {
                let my_size = core::mem::size_of::<Self>();
                let offset = senres.offset.get();
                if offset + my_size > senres.backing.as_slice().len() {
                    return Err(());
                }
                let val = Self::from_le_bytes(
                    senres.backing.as_slice()[offset..offset + my_size]
                        .try_into()
                        .unwrap(),
                );
                senres.offset.set(offset + my_size);
                Ok(val)
            }
        }
    };
}

impl<T: SenSer<Backing, N>, Backing: Senres, const N: usize> SenSer<Backing, N> for Option<T> {
    fn append_to(&self, senres: &mut SenresWriter<Backing, N>) {
        if let Some(val) = self {
            senres.append(1u8);
            val.append_to(senres);
        } else {
            senres.append(0u8);
        }
    }
}

impl<T: RecDes<Backing, N>, Backing: Senres, const N: usize> RecDes<Backing, N> for Option<T> {
    fn try_get_from(senres: &SenresReader<Backing, N>) -> Result<Self, ()> {
        if senres.offset.get() + 1 > senres.backing.as_slice().len() {
            return Err(());
        }
        let check = senres.try_get_from::<u8>()?;
        if check == 0 {
            return Ok(None);
        }
        if check != 1 {
            return Err(());
        }
        let my_size = core::mem::size_of::<Self>();
        if senres.offset.get() + my_size > senres.backing.as_slice().len() {
            return Err(());
        }
        Ok(Some(T::try_get_from(senres)?))
    }
}

primitive_impl! {u8}
primitive_impl! {i8}
primitive_impl! {u16}
primitive_impl! {i16}
primitive_impl! {u32}
primitive_impl! {i32}
primitive_impl! {u64}
primitive_impl! {i64}

impl<T: SenSer<Backing, N>, Backing: Senres, const N: usize> SenSer<Backing, N> for &[T] {
    fn append_to(&self, senres: &mut SenresWriter<Backing, N>) {
        senres.append(self.len() as u32);
        for entry in self.iter() {
            entry.append_to(senres)
        }
    }
}

impl<Backing: Senres, const N: usize> SenSer<Backing, N> for str {
    fn append_to(&self, senres: &mut SenresWriter<Backing, N>) {
        senres.append(self.len() as u32);
        for (src, dest) in self
            .as_bytes()
            .iter()
            .zip(senres.backing.as_mut_slice()[senres.offset..].iter_mut())
        {
            *dest = *src;
            senres.offset += 1;
        }
    }
}

impl<Backing: Senres, const N: usize> SenSer<Backing, N> for &str {
    fn append_to(&self, senres: &mut SenresWriter<Backing, N>) {
        senres.append(self.len() as u32);
        for (src, dest) in self
            .as_bytes()
            .iter()
            .zip(senres.backing.as_mut_slice()[senres.offset..].iter_mut())
        {
            *dest = *src;
            senres.offset += 1;
        }
    }
}

impl<Backing: Senres, const N: usize> RecDes<Backing, N> for String {
    fn try_get_from(senres: &SenresReader<Backing, N>) -> Result<Self, ()> {
        let len = senres.try_get_from::<u32>()? as usize;
        let offset = senres.offset.get();
        if offset + len > senres.backing.as_slice().len() {
            return Err(());
        }
        core::str::from_utf8(&senres.backing.as_slice()[offset..offset + len])
            .or(Err(()))
            .map(|e| {
                senres.offset.set(offset + len);
                e.to_owned()
            })
    }
}

impl<'a, Backing: Senres, const N: usize> RecDesRef<'a, Backing, N> for str {
    fn try_get_ref_from(senres: &'a SenresReader<Backing, N>) -> Result<&'a Self, ()> {
        let len = senres.try_get_from::<u32>()? as usize;
        let offset = senres.offset.get();
        if offset + len > senres.backing.as_slice().len() {
            return Err(());
        }
        core::str::from_utf8(&senres.backing.as_slice()[offset..offset + len])
            .or(Err(()))
            .map(|e| {
                senres.offset.set(offset + len);
                e
            })
    }
}

impl<'a, Backing: Senres, T: RecDes<Backing, N>, const N: usize> RecDesRef<'a, Backing, N>
    for [T]
{
    fn try_get_ref_from(senres: &'a SenresReader<Backing, N>) -> Result<&'a Self, ()> {
        let len = senres.try_get_from::<u32>()? as usize;
        let offset = senres.offset.get();
        if offset + (len * core::mem::size_of::<T>()) > senres.backing.as_slice().len() {
            return Err(());
        }
        let ret = unsafe {
            core::slice::from_raw_parts(
                senres.backing.as_slice().as_ptr().add(offset) as *const T,
                len,
            )
        };
        senres.offset.set(offset + len * core::mem::size_of::<T>());
        Ok(ret)
    }
}

impl<const N: usize> Default for SenresStack<N> {
    fn default() -> Self {
        Self::new()
    }
}

fn do_stuff(_r: &SenresStack) {
    println!("Stuff!");
}

#[test]
fn main() {
    let mut sr1 = SenresStack::<4096>::new();
    // let sr2 = Senres::<4097>::new();
    let sr3 = SenresStack::<8192>::new();
    // let sr4 = Senres::<4098>::new();
    let sr5 = SenresStack::new();

    do_stuff(&sr5);
    println!("Size of sr1: {}", core::mem::size_of_val(&sr1));
    println!("Size of sr3: {}", core::mem::size_of_val(&sr3));
    println!("Size of sr5: {}", core::mem::size_of_val(&sr5));

    {
        let mut writer = sr1.writer();
        writer.append(16777215u32);
        writer.append(u64::MAX);
        writer.append("Hello, world!");
        writer.append("String2");
        writer.append::<Option<u32>>(None);
        writer.append::<Option<u32>>(Some(42));
        writer.append([1i32, 2, 3, 4, 5].as_slice());
        writer.append([5u8, 4, 3, 2].as_slice());
        writer.append(["Hello", "There", "World"].as_slice());
    }
    println!("sr1: {:?}", sr1);

    {
        let reader = sr1.reader().expect("couldn't get reader");
        let val: u32 = reader.try_get_from().expect("couldn't get the u32 value");
        println!("u32 val: {}", val);
        let val: u64 = reader.try_get_from().expect("couldn't get the u64 value");
        println!("u64 val: {:x}", val);
        let val: &str = reader
            .try_get_ref_from()
            .expect("couldn't get string value");
        println!("String val: {}", val);
        let val: String = reader.try_get_from().expect("couldn't get string2 value");
        println!("String2 val: {}", val);
        let val: Option<u32> = reader.try_get_from().expect("couldn't get Option<u32>");
        println!("Option<u32> val: {:?}", val);
        let val: Option<u32> = reader.try_get_from().expect("couldn't get Option<u32>");
        println!("Option<u32> val: {:?}", val);
        let val: &[i32] = reader.try_get_ref_from().expect("couldn't get &[i32]");
        println!("&[i32] val: {:?}", val);
        let val: &[u8] = reader.try_get_ref_from().expect("couldn't get &[u8]");
        println!("&[u8] val: {:?}", val);
    }
}
