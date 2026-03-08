use std::mem::size_of;
pub mod ext;

const BTF_MAGIC: u16 = 0xeb9f;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BtfHeader {
    pub magic: u16,
    pub version: u8,
    pub flags: u8,
    pub hdr_len: u32,
    pub type_off: u32,
    pub type_len: u32,
    pub str_off: u32,
    pub str_len: u32,
}

pub struct BtfBuilder {
    types: Vec<u8>,
    strings: Vec<u8>,
}

impl BtfBuilder {
    pub fn new() -> Self {
        let mut strings = Vec::new();
        strings.push(0); // string table must start with empty string
        Self {
            types: Vec::new(),
            strings,
        }
    }

    pub fn add_string(&mut self, s: &str) -> u32 {
        let off = self.strings.len() as u32;
        self.strings.extend_from_slice(s.as_bytes());
        self.strings.push(0);
        off
    }

    pub fn emit(self) -> Vec<u8> {
        let header = BtfHeader {
            magic: BTF_MAGIC,
            version: 1,
            flags: 0,
            hdr_len: size_of::<BtfHeader>() as u32,
            type_off: 0,
            type_len: self.types.len() as u32,
            str_off: self.types.len() as u32,
            str_len: self.strings.len() as u32,
        };

        let mut out = Vec::new();

        unsafe {
            let ptr = &header as *const BtfHeader as *const u8;
            out.extend_from_slice(std::slice::from_raw_parts(ptr, size_of::<BtfHeader>()));
        }

        out.extend(self.types);
        out.extend(self.strings);

        out
    }
}