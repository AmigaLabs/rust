use crate::spec::{Cc, LinkerFlavor, Lld, Os, PanicStrategy, RelocModel, TargetOptions, TlsModel, cvs};

pub(crate) fn opts() -> TargetOptions {
    let mut base = TargetOptions {
        os: Os::Amigaos,
        families: cvs!["unix"],
        linker: Some("ppc-amigaos-gcc".into()),
        linker_flavor: LinkerFlavor::Gnu(Cc::Yes, Lld::No),
        dynamic_linking: true,
        relocation_model: RelocModel::Static,
        position_independent_executables: false,
        has_thread_local: false,
        tls_model: TlsModel::Emulated,
        crt_static_default: true,
        crt_static_respected: true,
        panic_strategy: PanicStrategy::Abort,
        ..Default::default()
    };
    base.add_pre_link_args(LinkerFlavor::Gnu(Cc::Yes, Lld::No), &["-mcrt=clib4", "-athread=native"]);
    base
}
