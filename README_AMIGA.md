# Rust for AmigaOS 4 (`powerpc-unknown-amigaos`)

Maintenance fork of Rust with the `powerpc-unknown-amigaos` target (tier 3), built against the clib4 C library. This work is not intended for rust-lang/rust. We keep it aligned with upstream ourselves: after syncing the fork on GitHub, rebase the `amigaos-port` branch onto `origin/main`.

## Status

Verified on the Amiga (on the previous upstream base; re-verification on the current base is pending):
- building and linking statically linked programs with std;
- processes: spawn, arguments with special characters, pipes for stdin/stdout/stderr, exit status, kill with SIGTERM;
- environment variables, basic file system, local sockets, clock;
- the `libc` crate through a patch (see Dependencies).

Not supported or not verified:
- SIGKILL: clib4 ignores it silently.
- DNS (`getaddrinfo`) is not tested.
- Panics and exit codes under cargo are not tested.
- `current_exe` uses `/proc`, which does not exist on AmigaOS.
- Thread names are ignored.
- `env_remove` and `env_clear` only affect the environment during spawn, and the change is visible to other threads.
- `Output` reads stdout and then stderr in sequence. If stderr fills up while stdout is still open, it can block (not reproduced up to 2 MB).
- std does not use `poll` on pipes, because it is unreliable on AmigaOS.
- The `dropping unsupported crate type dylib` warning is expected: the target is static, and dylib and cdylib are not supported.

## Dependencies

- **clib4**: fork `AmigaLabs/clib4`. The non-blocking read (EAGAIN) and argument quoting fixes are in PRs #481 and #482.
- **libc**: fork `AmigaLabs/libc` (branch `main`) of libc 0.2.189 with the AmigaOS module. `library/Cargo.toml` uses it in the `[patch.crates-io]` section, and `library/Cargo.lock` pins it to a specific commit.
- **backtrace**: submodule `library/backtrace`, pointing to `AmigaLabs/backtrace-rs` (branch `amigaos-port`). Its commit disables the libunwind backend on AmigaOS.

Every project that uses `libc` on AmigaOS needs the same `[patch.crates-io]` entry in its own `Cargo.toml`, with the same git reference.

## Prerequisites

- Linux with the usual Rust build dependencies (see the upstream development guide).
- Docker with access to the daemon. If your user is in the `docker` group but the current session does not have it active, use `sg docker -c "..."` or start a new session.
- Image `walkero/amigagccondocker:os4-gcc15-3.5.0-dwarf2`: gcc 15.3.0, binutils, and clib4 under `/opt/ppc-amigaos/ppc-amigaos/SDK/clib4`.

Wrapper variables:
- `AMIGA_GCC_IMAGE`: image to use (default: the one above).
- `DOCKER_CONTEXT`: Docker context (default: `default`).

## Build

From the repository root:

```bash
cp amiga/bootstrap.toml.example bootstrap.toml
export PATH="$PWD/amiga/bin:$PATH"
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build compiler/rustc
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build library/std --target powerpc-unknown-amigaos
```

`amiga/bin/ppc-amigaos-gcc` runs gcc inside the container, with the repository mounted at the same path as on the host. `amiga/bin/rustc-amiga` picks the stage1 compiler and sysroot for the AmigaOS target, and the stage0 compiler for host build scripts.

`bootstrap.toml` is ignored by git. Start from `amiga/bootstrap.toml.example`.

## Testing a program

Linking runs inside the container, so sources and output must live under the repository root:

```bash
mkdir -p build/amiga-test
cp hello.rs build/amiga-test/
cd build/amiga-test
rustc-amiga --target powerpc-unknown-amigaos hello.rs -o hello
```

The result is a statically linked ELF 32-bit big-endian executable. Copy it to the Amiga to run it.

## Syncing with upstream

- `origin` is `AmigaLabs/rust`, where pushes go.
- `upstream` is `rust-lang/rust`, with push disabled.
- To align: sync the fork on GitHub, then run `git fetch origin` and `git rebase origin/main` on `amigaos-port`.
- To compare with upstream: `git fetch upstream main`.

## Note on authorship

The code, comments, and this document were written with the help of an LLM (Claude). The port was verified by the author on the Amiga. It is not intended to be proposed to rust-lang/rust.
