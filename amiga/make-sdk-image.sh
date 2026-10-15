#!/bin/sh
# Builds a local cross-SDK image whose libc.a gains the objects from amiga/sdk:
# assembly stubs (getexecfullname) and C shims (preadv, pwritev, dup3) that
# clib4 lacks. Each run starts from a fresh container of the base image, so the
# script can be rerun safely.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CTX="${DOCKER_CONTEXT:-default}"
BASE="${AMIGA_BASE_IMAGE:-walkero/amigagccondocker:os4-gcc15-3.5.0-dwarf2}"
OUT="${AMIGA_SDK_IMAGE:-amigagcc-local:clib4-fullname}"
NAME="amiga-sdk-stub-$$"

docker --context "$CTX" rm -f "$NAME" >/dev/null 2>&1 || true
docker --context "$CTX" run --name "$NAME" -v "$ROOT/amiga/sdk:/stub:ro" "$BASE" sh -c '
    set -e
    LIBC=/opt/ppc-amigaos/ppc-amigaos/SDK/clib4/lib/libc.a
    BIN=/opt/ppc-amigaos/bin
    for src in /stub/*.s; do
        $BIN/ppc-amigaos-as -o /tmp/$(basename "$src" .s).o "$src"
    done
    for src in /stub/*.c; do
        $BIN/ppc-amigaos-gcc -mcrt=clib4 -O2 -c "$src" -o /tmp/$(basename "$src" .c).o
    done
    $BIN/ppc-amigaos-ar r "$LIBC" /tmp/*.o
    for sym in getexecfullname preadv pwritev dup3; do
        $BIN/ppc-amigaos-nm "$LIBC" | grep -q " T $sym$"
    done
'
docker --context "$CTX" commit "$NAME" "$OUT" >/dev/null
docker --context "$CTX" rm "$NAME" >/dev/null
echo "built $OUT from $BASE"
