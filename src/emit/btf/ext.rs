use std::mem::size_of;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BtfExtHeader {
    pub magic: u16,
    pub version: u8,
    pub flags: u8,
    pub hdr_len: u32,
    pub func_info_off: u32,
    pub func_info_len: u32,
    pub line_info_off: u32,
    pub line_info_len: u32,
}

pub fn build_btf_ext() -> Vec<u8> {
    let header = BtfExtHeader {
        magic: 0xeb9f,
        version: 1,
        flags: 0,
        hdr_len: size_of::<BtfExtHeader>() as u32,
        func_info_off: 0,
        func_info_len: 0,
        line_info_off: 0,
        line_info_len: 0,
    };

    let mut out = Vec::new();

    unsafe {
        let ptr = &header as *const BtfExtHeader as *const u8;
        out.extend_from_slice(std::slice::from_raw_parts(ptr, size_of::<BtfExtHeader>()));
    }

    out
}