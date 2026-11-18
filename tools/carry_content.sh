#!/usr/bin/env bash
# Re-carry the jar's binary assets into the package root (`assets/`), or check
# that what is there is byte-identical to the jar's own entries (`--check`).
#
# The JSON in data/ and assets/ is written by hand from the jar and the upstream classes
# (`FINDINGS.md` records every number and every shape decision); the textures and
# lang files are the jar's own bytes, and this script is how they got here, so a
# reviewer can re-run it.
#
#   tools/carry_content.sh           # copy them out of the jar
#   tools/carry_content.sh --check   # fail if any carried file differs
set -Eeuo pipefail

PORT=$(cd "$(dirname "$0")/.." && pwd)
CAMPAIGN=$(cd "$PORT/../.." && pwd)
JAR=${JAR:-$CAMPAIGN/ports/_shared/licence-cache/dysoncubeproject-1.0.5.jar}
ID=dysoncubeproject
CHECK=0
[ "${1:-}" = "--check" ] || [ $# -eq 0 ] || { echo "usage: $0 [--check]" >&2; exit 2; }
[ "${1:-}" = "--check" ] && CHECK=1

[ -f "$JAR" ] || { echo "missing jar: $JAR" >&2; exit 1; }

# Every entry taken from the jar verbatim.
ENTRIES=(
    "assets/$ID/textures/block/ray_receiver_base.png"
    "assets/$ID/textures/block/ray_receiver_plate.png"
    "assets/$ID/textures/block/particle.png"
    "assets/$ID/textures/item/beam.png"
    "assets/$ID/textures/item/beam_package.png"
    "assets/$ID/textures/item/solar_sail.png"
    "assets/$ID/textures/item/solar_sail_package.png"
    "assets/$ID/lang/en_us.json"
    "assets/$ID/lang/ja_jp.json"
    "assets/$ID/lang/pt_br.json"
    "assets/$ID/lang/ru_ru.json"
    "assets/$ID/lang/tr_tr.json"
    "assets/$ID/lang/zh_cn.json"
)

status=0
for entry in "${ENTRIES[@]}"; do
    target="$PORT/$entry"
    if [ "$CHECK" = 1 ]; then
        if ! cmp -s <(unzip -p "$JAR" "$entry") "$target"; then
            echo "DIFFERS: $entry" >&2
            status=1
        fi
    else
        mkdir -p "$(dirname "$target")"
        unzip -p "$JAR" "$entry" > "$target"
        echo "carried $entry"
    fi
done

if [ "$CHECK" = 1 ]; then
    [ "$status" = 0 ] && echo "assets/ matches the jar for ${#ENTRIES[@]} carried entries"
else
    echo "carried ${#ENTRIES[@]} entries from $(basename "$JAR")"
fi
exit "$status"
