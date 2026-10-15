#!/bin/sh
# Builds a local cross-SDK image whose libc.a exports clib4's getexecfullname().
# The stub from amiga/sdk is added to a fresh container of the base image, so
# the script can be rerun safely.
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
    /opt/ppc-amigaos/bin/ppc-amigaos-as -o /tmp/getexecfullname_stub.o /stub/getexecfullname_stub.s
    /opt/ppc-amigaos/bin/ppc-amigaos-ar r "$LIBC" /tmp/getexecfullname_stub.o
    /opt/ppc-amigaos/bin/ppc-amigaos-nm "$LIBC" | grep -q " T getexecfullname$"
'
docker --context "$CTX" commit "$NAME" "$OUT" >/dev/null
docker --context "$CTX" rm "$NAME" >/dev/null
echo "built $OUT from $BASE"
