#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CtxMethod {
    // ================================
    // Context memory access
    // ================================
    LoadU8,
    LoadU16,
    LoadU32,
    LoadU64,
    LoadI8,
    LoadI16,
    LoadI32,
    LoadI64,
    LoadBytes,

    // ================================
    // Process / task metadata helpers
    // ================================
    GetPidTgid,     // bpf_get_current_pid_tgid (helper 14)
    GetUidGid,      // bpf_get_current_uid_gid
    GetCurrentComm, // bpf_get_current_comm
    GetCurrentTask, // bpf_get_current_task

    // ================================
    // Time helpers
    // ================================
    GetKtimeNs, // bpf_ktime_get_ns

    // ================================
    // Memory probe helpers
    // ================================
    ProbeReadUserStr,   // bpf_probe_read_user_str (helper 202)
    ProbeReadKernelStr, // bpf_probe_read_kernel_str (helper 204)
}

impl CtxMethod {
    pub fn from_str(name: &str) -> Option<Self> {
        match name {
            // Context loads
            "load_u8" => Some(Self::LoadU8),
            "load_u16" => Some(Self::LoadU16),
            "load_u32" => Some(Self::LoadU32),
            "load_u64" => Some(Self::LoadU64),
            "load_i8" => Some(Self::LoadI8),
            "load_i16" => Some(Self::LoadI16),
            "load_i32" => Some(Self::LoadI32),
            "load_i64" => Some(Self::LoadI64),
            "load_bytes" => Some(Self::LoadBytes),

            // Metadata helpers
            "get_pid_tgid" => Some(Self::GetPidTgid),
            "get_uid_gid" => Some(Self::GetUidGid),
            "get_current_comm" => Some(Self::GetCurrentComm),
            "get_current_task" => Some(Self::GetCurrentTask),

            // Time helpers
            "get_ktime_ns" => Some(Self::GetKtimeNs),

            // Memory probe helpers
            "probe_read_user_str" => Some(Self::ProbeReadUserStr),
            "probe_read_kernel_str" => Some(Self::ProbeReadKernelStr),

            _ => None,
        }
    }
}
