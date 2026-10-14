# Rust per AmigaOS 4 (`powerpc-unknown-amigaos`)

Fork di manutenzione di Rust con il target `powerpc-unknown-amigaos` (tier 3), compilato con la libc clib4. Il lavoro non è destinato a rust-lang/rust: l'allineamento con l'upstream lo facciamo noi, rebasando il branch `amigaos-port` su `origin/main` dopo il Sync del fork.

## Stato

Verificato sull'Amiga:
- compilazione e link di programmi statici con std;
- processi: spawn, argomenti con caratteri speciali, pipe per stdin/stdout/stderr, exit status, kill con SIGTERM;
- variabili d'ambiente, file system di base, socket locali, clock;
- crate `libc` tramite patch (vedi Dipendenze).

Non supportato o non verificato:
- SIGKILL: clib4 lo ignora senza errore.
- DNS (`getaddrinfo`) non testato.
- Panic e codice di uscita sotto cargo non testati.
- `current_exe` usa `/proc`, che su AmigaOS non esiste.
- I nomi dei thread sono ignorati.
- `env_remove` ed `env_clear` agiscono sull'ambiente solo durante lo spawn, e la modifica è visibile agli altri thread.
- `Output` legge stdout e poi stderr in sequenza: se stderr si riempie mentre stdout è ancora aperto, può bloccarsi (non riprodotto fino a 2 MB).
- std non usa `poll` sulle pipe, perché su AmigaOS non è affidabile.
- L'avviso `dropping unsupported crate type dylib` è atteso: il target è statico e dylib/cdylib non sono supportati.

## Dipendenze

- **clib4**: fork `AmigaLabs/clib4`. Le modifiche per la lettura non bloccante (EAGAIN) e il quoting degli argomenti sono nelle PR #481 e #482.
- **libc**: fork della libc 0.2.189 con il modulo AmigaOS. Il branch locale ha la storia `Import libc 0.2.189` + `Add AmigaOS (clib4) module`. Va pubblicato come `AmigaLabs/libc`. Finché non è pubblicato, `library/Cargo.toml` usa un percorso assoluto nella patch `[patch.crates-io]`, che va sostituito con il riferimento git.
- **backtrace**: il sottomodulo `library/backtrace` richiede un commit locale che disabilita il backend libunwind su AmigaOS. Va pubblicato come `AmigaLabs/backtrace-rs` e il sottomodulo va puntato lì.

Ogni progetto che usa `libc` su AmigaOS deve avere la stessa patch `[patch.crates-io]` nel proprio `Cargo.toml`.

## Prerequisiti

- Linux con le dipendenze di build di Rust (vedi la guida di sviluppo upstream).
- Docker con accesso al daemon. Se il tuo utente è nel gruppo `docker` ma la sessione corrente non lo ha attivo, usa `sg docker -c "..."` oppure riapri la sessione.
- Immagine `walkero/amigagccondocker:os4-gcc15-3.5.0-dwarf2`: gcc 15.3.0, binutils e clib4 in `/opt/ppc-amigaos/ppc-amigaos/SDK/clib4`.

Variabili dei wrapper:
- `AMIGA_GCC_IMAGE`: immagine da usare (default: quella sopra).
- `DOCKER_CONTEXT`: contesto Docker (default: `default`).

## Build

Dalla radice del repo:

```bash
cp amiga/bootstrap.toml.example bootstrap.toml
export PATH="$PWD/amiga/bin:$PATH"
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build compiler/rustc
BOOTSTRAP_SKIP_TARGET_SANITY=1 ./x build library/std --target powerpc-unknown-amigaos
```

`amiga/bin/ppc-amigaos-gcc` esegue gcc nel container, con il repo montato allo stesso percorso dell'host. `amiga/bin/rustc-amiga` sceglie il compilatore stage1 e il sysroot per il target AmigaOS, e lo stage0 per gli script di build dell'host.

Il `bootstrap.toml` è ignorato da git: parte dal file `amiga/bootstrap.toml.example`.

## Test di un programma

Il link passa dal container, quindi sorgenti e output devono stare sotto la radice del repo:

```bash
mkdir -p build/amiga-test
cp hello.rs build/amiga-test/
cd build/amiga-test
rustc-amiga --target powerpc-unknown-amigaos hello.rs -o hello
```

Il binario risultante è ELF 32-bit big-endian statico, da copiare sull'Amiga.

## Sincronizzazione con l'upstream

- `origin` è `AmigaLabs/rust`, dove si fa il push.
- `upstream` è `rust-lang/rust`, con il push disabilitato.
- Per allinearsi: fare il Sync del fork su GitHub, poi `git fetch origin` e `git rebase origin/main` su `amigaos-port`.
- Per confrontare con l'upstream: `git fetch upstream main`.

## Nota sulla scrittura

Il codice, i commenti e questo documento sono stati scritti con l'assistenza di un LLM (Claude). Il port è stato verificato dall'autore sull'Amiga. Non è destinato a essere proposto a rust-lang/rust.
