use rustc_abi::Endian;

use crate::spec::{Arch, Target, TargetMetadata, TargetOptions, base};

pub(crate) fn target() -> Target {
    let mut base = base::amigaos::opts();
    base.max_atomic_width = Some(32);

    Target {
        llvm_target: "powerpc-unknown-amigaos".into(),
        metadata: TargetMetadata {
            description: Some("PowerPC AmigaOS 4 (clib4)".into()),
            tier: Some(3),
            host_tools: Some(false),
            std: Some(true),
        },
        pointer_width: 32,
        data_layout: "E-m:e-p:32:32-Fn32-i64:64-n32".into(),
        arch: Arch::PowerPC,
        options: TargetOptions { endian: Endian::Big, ..base },
    }
}
