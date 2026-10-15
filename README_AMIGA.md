# Rust for AmigaOS 4 (`powerpc-unknown-amigaos`)

Maintenance fork of Rust with the `powerpc-unknown-amigaos` target (tier 3), built against the clib4 C library. This work is not intended for rust-lang/rust. We keep it aligned with upstream ourselves: after syncing the fork on GitHub, rebase the `amigaos-port` branch onto `origin/main`.

## Status

Verified on the Amiga (process, file, and network tests on the previous upstream base; re-verification on the current base is pending, except DNS and `current_exe()`, which were tested on the current base):
- building and linking statically linked programs with std;
- processes: spawn, arguments with special characters, pipes for stdin/stdout/stderr, exit status, kill with SIGTERM;
- environment variables, basic file system, local sockets, clock;
- DNS: `getaddrinfo` resolves `localhost`, IP literals, and public names, and reports unknown names as errors;
- `env::current_exe()` returns the full path of the program (`Work:dir/prog`) when started from the shell with a bare name, a relative path, or an absolute path, from any current directory;
- the `libc` crate through a patch (see Dependencies);
- `cargo test`: test binaries built for the target run on the Amiga, covering unit tests, file I/O, threads, and a child process spawned from a test thread with its output read through a pipe.
- Stack: every executable carries a 1 MB `$STACK:` cookie, added by std (`library/std/src/rt.rs`). AmigaOS uses a much smaller default stack, and ordinary Rust code such as `clap` overflowed it.
- Paths: std rewrites paths for clib4 (`library/std/src/sys/helpers/amigaos_path.rs`): `.`, `..`, and unix absolute paths such as `/Work/dir` become AmigaDOS names, and `current_dir()` returns `Work:dir`. The result is the same with and without unix path semantics.
- File types: directory entries report their type correctly. clib4's `d_type` values are used on AmigaOS (`DT_REG = 1`), not Linux's.
- `tokei` 15 counts files and lines in a directory tree, with and without unix path semantics.

Not supported or not verified:
- SIGKILL: clib4 ignores it silently.
- Panics abort the program (`panic=abort`), so a failing cargo test aborts the whole test binary instead of being reported. `#[should_panic]` tests are not supported.
- `current_exe()` for a program found through the `C:` search path, and for Workbench-started programs, is not tested.
- Thread names are ignored.
- `env_remove` and `env_clear` only affect the environment during spawn, and the change is visible to other threads.
- `Output` reads stdout and then stderr in sequence. If stderr fills up while stdout is still open, it can block (not reproduced up to 2 MB).
- std does not use `poll` on pipes, because it is unreliable on AmigaOS.
- The `dropping unsupported crate type dylib` warning is expected: the target is static, and dylib and cdylib are not supported.

## Dependencies

- **clib4**: fork `AmigaLabs/clib4`. The non-blocking read (EAGAIN) and argument quoting fixes are in PRs #481 and #482. Three local commits are not yet in `AmigaLabs/clib4`. Push them before building against it:
  - `getexecfullname()`, needed by `current_exe()`. It must be built into both `clib4.library` on the Amiga and the SDK used for linking.
  - The fix in `library/misc/children.c` (`spawnedProcessExit`). Without it, the exit status of a child spawned from a thread reads as "still running" (`0x10000000`), which breaks `cargo test` as soon as a test spawns a process. Only `clib4.library` needs replacing for this one.
  - `preadv()`, `pwritev()`, `dup3()`, and placeholder definitions for Linux names clib4 does not implement (termios flags and control characters, errno values from 200, and some baud rates). Commit `54e84c71`. The placeholders compile but do nothing; see the comments in the headers.

Until the SDK ships these symbols, `amiga/make-sdk-image.sh` adds the assembly stub and C shims in `amiga/sdk`. Remove those shims once the SDK is rebuilt from clib4 that includes this commit, or the link fails with duplicate symbols.
- **libc**: fork `AmigaLabs/libc` (branch `main`) of libc 0.2.189 with the AmigaOS module. `library/Cargo.toml` uses it in the `[patch.crates-io]` section, and `library/Cargo.lock` pins it to a specific commit.
- **backtrace**: submodule `library/backtrace`, pointing to `AmigaLabs/backtrace-rs` (branch `amigaos-port`). Its commit disables the libunwind backend on AmigaOS.

Every project that uses `libc` on AmigaOS needs the same `[patch.crates-io]` entry in its own `Cargo.toml`, with the same git reference.

## Prerequisites

- Linux with the usual Rust build dependencies (see the upstream development guide).
- Docker with access to the daemon. If your user is in the `docker` group but the current session does not have it active, use `sg docker -c "..."` or start a new session.
- Image `walkero/amigagccondocker:os4-gcc15-3.5.0-dwarf2`: gcc 15.3.0, binutils, and clib4 under `/opt/ppc-amigaos/ppc-amigaos/SDK/clib4`. This image lacks some symbols that clib4 does not provide: `getexecfullname` (needed by `current_exe()`), and `preadv`, `pwritev`, and `dup3`. Build the local SDK image once with `amiga/make-sdk-image.sh`, which adds them from `amiga/sdk` (assembly stub and C shims built on clib4's `readv`, `writev`, `dup2`, and `lseek`) and produces `amigagcc-local:clib4-fullname`. Set `AMIGA_GCC_IMAGE` to that image.

Wrapper variables:
- `AMIGA_GCC_IMAGE`: image to use (default: the one above).
- `DOCKER_CONTEXT`: Docker context (default: `default`).

## Build

From the repository root:

```bash
cp amiga/bootstrap.toml.example bootstrap.toml
export PATH="$PWD/amiga/bin:$PATH"
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build compiler/rustc
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build library/std library/proc_macro library/test --target x86_64-unknown-linux-gnu,powerpc-unknown-amigaos
```

Build all the sysroot targets in one `./x build` invocation. Each invocation rebuilds the stage1 sysroot only for the targets it names, so a separate build for the host would remove the AmigaOS libraries.

`amiga/bin/ppc-amigaos-gcc` runs gcc inside the container, with the repository mounted at the same path as on the host. `amiga/bin/rustc-amiga` always runs the stage1 compiler with its sysroot, for host and target builds alike: a proc-macro built by stage0 cannot be loaded by stage1 (E0514), so cargo must not mix them.

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
