#![allow(unused)]
/// A struct to send and receive data
#[repr(C, align(4096))]
#[derive(Debug)]
pub struct Senres<const N: usize = 4096> {
    data: [u8; N],
    offset: usize,
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

pub trait SenSer<const N: usize> {
    fn append_to(&self, senres: &mut Senres<N>);
}

pub trait RecDes<const N: usize> {
    fn try_get_from(senres: &mut Senres<N>) -> Result<Self, ()>
    where
        Self: std::marker::Sized;
}

pub trait RecDesRef<'a, const N: usize> {
    fn try_get_ref_from(senres: &'a mut Senres<N>) -> Result<&'a Self, ()>;
}

impl<const N: usize> Senres<N> {
    /// Ensure that `N` is a multiple of 4096. This constant should
    /// be evaluated in the constructor function.
    const CHECK_ALIGNED: () = if N & 4095 != 0 {
        panic!("Senres size must be a multiple of 4096")
    };

    pub const fn new() -> Self {
        // Ensure the `N` that was specified is a multiple of 4096
        #[allow(clippy::no_effect)]
        Self::CHECK_ALIGNED;
        Senres {
            data: [0u8; N],
            offset: 0,
            valid: 0,
        }
    }

    pub fn append<T: SenSer<N>>(&mut self, other: T) {
        other.append_to(self);
    }

    pub fn try_get_from<T: RecDes<N>>(&mut self) -> Result<T, ()> {
        T::try_get_from(self)
    }

    pub fn try_get_ref_from<'a, T: RecDesRef<'a, N> + ?Sized>(&'a mut self) -> Result<&'a T, ()> {
        T::try_get_ref_from(self)
    }

    #[cfg(not(target_os = "xous"))]
    pub fn lend(&self, connection: u32, opcode: usize) -> Result<(), ()> {
        Ok(())
    }

    #[cfg(not(target_os = "xous"))]
    pub fn lend_mut(mut self, connection: u32, opcode: usize) -> Result<Self, ()> {
        // let (offset, valid) = self.invoke(connection, opcode, InvokeType::LendMut)?;
        self.offset = 0;
        self.valid = 0;
        Ok(self)
    }

    pub(crate) fn rewind(&mut self) {
        self.offset = 0;
    }

    #[cfg(target_os = "xous")]
    pub fn lend(&self, connection: u32, opcode: usize) -> Result<(), ()> {
        let mut a0 = Syscall::SendMessage as usize;
        let mut a1: usize = connection.try_into().unwrap();
        let mut a2 = opcode;
        let mut a3 = InvokeType::Lend as usize;
        let mut a4 = &self.data as *const _ as usize;
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
        let mut a4 = &mut self.data as *mut _ as usize;
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

macro_rules! primitive_impl {
    ($SelfT:ty) => {
        impl<const N: usize> SenSer<N> for $SelfT {
            fn append_to(&self, senres: &mut Senres<N>) {
                for (src, dest) in self
                    .to_le_bytes()
                    .iter()
                    .zip(senres.data[senres.offset..].iter_mut())
                {
                    *dest = *src;
                    senres.offset += 1;
                }
            }
        }

        impl<const N: usize> RecDes<N> for $SelfT {
            fn try_get_from(senres: &mut Senres<N>) -> Result<Self, ()> {
                let my_size = core::mem::size_of::<Self>();
                if senres.offset + my_size > senres.data.len() {
                    return Err(());
                }
                let val = Self::from_le_bytes(
                    senres.data[senres.offset..senres.offset + my_size]
                        .try_into()
                        .unwrap(),
                );
                senres.offset += my_size;
                Ok(val)
            }
        }
    };
}

impl<T: SenSer<N>, const N: usize> SenSer<N> for Option<T> {
    fn append_to(&self, senres: &mut Senres<N>) {
        if let Some(val) = self {
            senres.append(1u8);
            val.append_to(senres);
        } else {
            senres.append(0u8);
        }
    }
}

impl<T: RecDes<N>, const N: usize> RecDes<N> for Option<T> {
    fn try_get_from(senres: &mut Senres<N>) -> Result<Self, ()> {
        if senres.offset + 1 > senres.data.len() {
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
        if senres.offset + my_size > senres.data.len() {
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

impl<T: SenSer<N>, const N: usize> SenSer<N> for &[T] {
    fn append_to(&self, senres: &mut Senres<N>) {
        senres.append(self.len() as u32);
        for entry in self.iter() {
            entry.append_to(senres)
        }
    }
}

impl<const N: usize> SenSer<N> for str {
    fn append_to(&self, senres: &mut Senres<N>) {
        senres.append(self.len() as u32);
        for (src, dest) in self
            .as_bytes()
            .iter()
            .zip(senres.data[senres.offset..].iter_mut())
        {
            *dest = *src;
            senres.offset += 1;
        }
    }
}

impl<const N: usize> SenSer<N> for &str {
    fn append_to(&self, senres: &mut Senres<N>) {
        senres.append(self.len() as u32);
        for (src, dest) in self
            .as_bytes()
            .iter()
            .zip(senres.data[senres.offset..].iter_mut())
        {
            *dest = *src;
            senres.offset += 1;
        }
    }
}

impl<const N: usize> RecDes<N> for String {
    fn try_get_from(senres: &mut Senres<N>) -> Result<Self, ()> {
        let len = senres.try_get_from::<u32>()? as usize;
        if senres.offset + len > senres.data.len() {
            return Err(());
        }
        core::str::from_utf8(&senres.data[senres.offset..senres.offset + len])
            .or(Err(()))
            .map(|e| {
                senres.offset += len;
                e.to_owned()
            })
    }
}

impl<'a, const N: usize> RecDesRef<'a, N> for str {
    fn try_get_ref_from(senres: &'a mut Senres<N>) -> Result<&'a Self, ()> {
        let len = senres.try_get_from::<u32>()? as usize;
        if senres.offset + len > senres.data.len() {
            return Err(());
        }
        core::str::from_utf8(&senres.data[senres.offset..senres.offset + len])
            .or(Err(()))
            .map(|e| {
                senres.offset += len;
                e
            })
    }
}

impl<'a, T: RecDes<N>, const N: usize> RecDesRef<'a, N> for [T] {
    fn try_get_ref_from(senres: &'a mut Senres<N>) -> Result<&'a Self, ()> {
        let len = senres.try_get_from::<u32>()? as usize;
        if senres.offset + (len * core::mem::size_of::<T>()) > senres.data.len() {
            return Err(());
        }
        let ret = unsafe {
            core::slice::from_raw_parts(senres.data.as_ptr().add(senres.offset) as *const T, len)
        };
        senres.offset += len * core::mem::size_of::<T>();
        Ok(ret)
    }
}

impl<const N: usize> Default for Senres<N> {
    fn default() -> Self {
        Self::new()
    }
}

fn do_stuff(_r: &Senres) {
    println!("Stuff!");
}

#[test]
fn main() {
    let mut sr1 = Senres::<4096>::new();
    // let sr2 = Senres::<4097>::new();
    let sr3 = Senres::<8192>::new();
    // let sr4 = Senres::<4098>::new();
    let sr5 = Senres::new();

    do_stuff(&sr5);
    println!("Size of sr1: {}", core::mem::size_of_val(&sr1));
    println!("Size of sr3: {}", core::mem::size_of_val(&sr3));
    println!("Size of sr5: {}", core::mem::size_of_val(&sr5));

    sr1.append(16777215u32);
    sr1.append(u64::MAX);
    sr1.append("Hello, world!");
    sr1.append("String2");
    sr1.append::<Option<u32>>(None);
    sr1.append::<Option<u32>>(Some(42));
    sr1.append([1i32, 2, 3, 4, 5].as_slice());
    sr1.append([5u8, 4, 3, 2].as_slice());
    sr1.append(["Hello", "There", "World"].as_slice());
    println!("sr1: {:?}", sr1);

    sr1.rewind();
    let val: u32 = sr1.try_get_from().expect("couldn't get the u32 value");
    println!("u32 val: {}", val);
    let val: u64 = sr1.try_get_from().expect("couldn't get the u64 value");
    println!("u64 val: {:x}", val);
    let val: &str = sr1.try_get_ref_from().expect("couldn't get string value");
    println!("String val: {}", val);
    let val: String = sr1.try_get_from().expect("couldn't get string2 value");
    println!("String2 val: {}", val);
    let val: Option<u32> = sr1.try_get_from().expect("couldn't get Option<u32>");
    println!("Option<u32> val: {:?}", val);
    let val: Option<u32> = sr1.try_get_from().expect("couldn't get Option<u32>");
    println!("Option<u32> val: {:?}", val);
    let val: &[i32] = sr1.try_get_ref_from().expect("couldn't get &[i32]");
    println!("&[i32] val: {:?}", val);
    let val: &[u8] = sr1.try_get_ref_from().expect("couldn't get &[u8]");
    println!("&[u8] val: {:?}", val);
}
