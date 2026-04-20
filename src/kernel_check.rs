use std::process::Command;
use std::fmt;

/// Minimum kernel version for CO-RE: 5.2.0
const MIN_KERNEL_MAJOR: u32 = 5;
const MIN_KERNEL_MINOR: u32 = 2;
const MIN_KERNEL_PATCH: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl KernelVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse from uname -r output like "5.10.134-18.0.9.lifsea8.x86_64"
    pub fn from_uname(release: &str) -> Option<Self> {
        let mut parts = release.split(|c: char| !c.is_ascii_digit());
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Some(Self::new(major, minor, patch))
    }

    pub fn is_core_supported(&self) -> bool {
        *self >= Self::new(MIN_KERNEL_MAJOR, MIN_KERNEL_MINOR, MIN_KERNEL_PATCH)
    }

    pub fn min_required() -> Self {
        Self::new(MIN_KERNEL_MAJOR, MIN_KERNEL_MINOR, MIN_KERNEL_PATCH)
    }
}

impl fmt::Display for KernelVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Detect kernel version from `uname -r`
pub fn detect_kernel_version() -> Result<KernelVersion, String> {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .map_err(|e| format!("Failed to run `uname -r`: {e}"))?;

    if !output.status.success() {
        return Err("`uname -r` returned non-zero".into());
    }

    let release = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    KernelVersion::from_uname(&release)
        .ok_or_else(|| format!("Cannot parse kernel version from: '{release}'"))
}

/// Check BTF availability (CO-RE requires kernel BTF)
pub fn check_btf_available() -> bool {
    std::path::Path::new("/sys/kernel/btf/vmlinux").exists()
}

/// Full CO-RE compatibility check
pub fn check_core_support() -> Result<(), CoreCheckError> {
    let version = detect_kernel_version()
        .map_err(CoreCheckError::VersionDetectionFailed)?;

    if !version.is_core_supported() {
        return Err(CoreCheckError::KernelTooOld {
            current: version,
            required: KernelVersion::min_required(),
        });
    }

    if !check_btf_available() {
        return Err(CoreCheckError::BtfMissing);
    }

    Ok(())
}

#[derive(Debug)]
pub enum CoreCheckError {
    VersionDetectionFailed(String),
    KernelTooOld {
        current: KernelVersion,
        required: KernelVersion,
    },
    BtfMissing,
}

impl fmt::Display for CoreCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreCheckError::VersionDetectionFailed(e) => {
                write!(f, "Cannot detect kernel version: {e}")
            }
            CoreCheckError::KernelTooOld { current, required } => {
                write!(f, 
                    "Kernel {current} does not support CO-RE.\n\
                     Minimum required: Linux {required}\n\
                     \n\
                     CO-RE requires kernel BTF which is available from Linux 5.2+.\n\
                     Upgrade your kernel or use a distribution with BTF-enabled builds."
                )
            }
            CoreCheckError::BtfMissing => {
                write!(f, 
                    "Kernel BTF not found at /sys/kernel/btf/vmlinux.\n\
                     \n\
                     Your kernel may be too old, or BTF was disabled at compile time.\n\
                     To fix:\n\
                       - Upgrade to Linux 5.2+ with CONFIG_DEBUG_INFO_BTF=y\n\
                       - Or install BTF files via: https://github.com/aquasecurity/btfhub"
                )
            }
        }
    }
}

impl std::error::Error for CoreCheckError {}