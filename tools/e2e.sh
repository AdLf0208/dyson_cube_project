#!/usr/bin/env bash
# In-game check for Dyson Cube Project slice 1: the ray receiver and its multiblock.
#
# **The game must be built by the launcher with this package** (mod API 0.2):
#
#   horizon-launcher native-build ports/dysoncubeproject --instance <instance> --trust-native-code
#   GAME=<that instance's horizon-client> tools/e2e.sh
#
# There is no `--dev-mod` any more: a native `.hmod` carries its code *and* its
# content, and the launcher compiles it against the pinned build. `horizon-launcher`
# is not installed in this campaign environment, so the 0.2.0 package has no in-game
# run yet — the logs this script produced for the retired 0.1.0 `.hnmod` are
# historical evidence for the byte-identical `runtime.rs`, not for the package
# (`../_shared/evidence/dysoncubeproject/HISTORICAL.md`).
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs the
# console path and bypasses the native command hook.
#
# The Overworld (the only level whose tick generates sphere power upstream), high
# over the terrain at x=100, z=0: a stone platform at y=179, the controller at
# (100,180,0) and the sixteen placeholders around it. `time set 1000` and
# `weather clear` keep the sky's conditions fixed.
#
# Default run:
#  A  control "wrong shape": a stone inside the 3x6x3 box makes the placement
#     refuse — the block stays air and no placeholder appears;
#  B  the same placement with a clear box forms: the controller is there and
#     sixteen `multiblock_structure` placeholders are written;
#  C  the readout: formed, 16/16, an empty sphere (beams 0, sails 0/0), no FE;
#  D  `/dysoncubeproject set beams` then `set panels`: the sail ceiling follows
#     the beams (8,333,334 beams -> 50,000,000 sails);
#  E  100 sails make 2,000 FE/t and the buffer grows at exactly that rate
#     (measured over the `t=` field, because a status round trip costs ticks);
#  F  control "night": at t=18000 the buffer stops growing; at t=1000 it grows
#     again at the same rate;
#  G  the caps: 50,000,000 sails generate 1,000,000,000 FE/t but the buffer
#     stops at its 100,000,000 FE capacity and nothing leaves (no FE sink below);
#  H  a broken placeholder takes the controller with it;
#  I  a broken controller clears its placeholders.
#
# CONTROL_GAME=<a launcher-built game without this package> adds the leg that makes
# the scenario fail-capable against another binary: in it neither the controller item
# nor either block id exists, so the formation assertions cannot be satisfied by a
# game that merely happens to hold the blocks. Unset, that leg is skipped.
#
# SELFTEST=1 flips E's rate assertion (it must FAIL), proving the scenario can fail.
set -Eeuo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/dysoncubeproject
# The launcher-built game carrying this package. Required.
GAME=${GAME:-}
# A launcher-built game *without* this package, for the control leg (optional).
CONTROL_GAME=${CONTROL_GAME:-}
# The control client is protocol-compatible at the pin; override with CTL= for another.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/dysoncubeproject-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=DysonCubeProjectE2E
CONTROL_WORLD=DysonCubeProjectNoMod

[ -n "$GAME" ] || {
    echo "missing GAME: this package needs a launcher-built game (see the header)" >&2
    exit 2
}

# The controller in the Overworld's sky: X,Y,Z is its own cell.
X=100
Y=180
Z=0
CONTROLLER="$X $Y $Z"
BELOW="$X $((Y - 1)) $Z"
FLOOR_Y=$((Y - 1))
# A cell inside the 3x6x3 box (x-1..x+1, y..y+5, z-1..z+1) that is not a placeholder.
BLOCKER="$X $((Y + 3)) $Z"

[ -x "$GAME" ] || { echo "missing launcher-built horizon-client: $GAME" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

# `/tmp/opencode` is the campaign's scratch root; the second machine starts
# without it, so make it (idempotent) before the unique folder.
mkdir -p /tmp/opencode
SCRATCH=$(mktemp -d /tmp/opencode/dysoncubeproject-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

cleanup() {
    status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD] [GAME]
    local world=${1:-$WORLD}
    local game=${2:-$GAME}
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch --game "$game" --world "$world" --seed 20261105 \
        --creative --headless --wait --resources "$RES" >/dev/null
    ctl wait-ready 120 >/dev/null
    ctl wait 40 >/dev/null
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
count_of() { # count_of BLOCK X1 Y1 Z1 X2 Y2 Z2
    ctl blocks "$2" "$3" "$4" "$5" "$6" "$7" | jq -r --arg b "$1" '.counts[$b] // 0'
}

# A stone platform at y=179 in the Overworld's sky, clear air above it.
arena() {
    ctl command "execute in minecraft:overworld run tp @s $((X + 2)).5 $Y $((Z + 0)).5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X" "$FLOOR_Y" "$Z" | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the arena chunk never loaded"
    ctl command "execute in minecraft:overworld run fill $((X - 6)) $FLOOR_Y $((Z - 6)) $((X + 6)) $FLOOR_Y $((Z + 6)) minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:overworld run fill $((X - 6)) $Y $((Z - 6)) $((X + 6)) $((Y + 8)) $((Z + 6)) minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:overworld run kill @e[type=!player]" >/dev/null
    ctl command "time set 1000" >/dev/null
    ctl command "weather clear 1000000" >/dev/null
    ctl command "execute in minecraft:overworld run tp @s $((X + 2)).5 $Y $((Z + 0)).5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X" "$FLOOR_Y" "$Z")" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X" "$Y" "$Z")" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

place_controller() { # place_controller: hold the controller and place it at (X,Y,Z)
    ctl give dysoncubeproject:ray_receiver_controller 1 >/dev/null || true
    ctl wait 4 >/dev/null
    ctl select dysoncubeproject:ray_receiver_controller >/dev/null
    ctl wait 2 >/dev/null
    ctl place "$X" "$Y" "$Z" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
}

# status X Y Z: this block's status line, after a fresh reply arrives (the `t=`
# field makes each reply unique).
status() { # status X Y Z
    local out line before
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E "Ray Receiver at $1,$2,$3:|No ray receiver" | head -1)
    say "dysoncubeproject status $1 $2 $3"
    for _ in $(seq 1 25); do
        ctl wait 4 >/dev/null
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E "Ray Receiver at $1,$2,$3:|No ray receiver" | head -1)
        if [ -n "$line" ] && [ "$line" != "$before" ]; then
            echo "$line"
            return
        fi
    done
    echo "NO-STATUS $(jq -c '{chat,action_bar}' <<<"$out")"
}

# change FORM: send one of upstream's `set|add beams|panels` forms, then read the
# status back.
change() { # change FORM ARGS
    local form=$1
    shift
    say "dysoncubeproject $form $*"
    ctl wait 4 >/dev/null
}

buffer_of() { sed -nE 's/.* buffer ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
gen_of() { sed -nE 's/.* gen ([0-9]+) FE\/t.*/\1/p' <<<"$1"; }
con_of() { sed -nE 's/.* con ([0-9]+) FE\/t.*/\1/p' <<<"$1"; }
beams_of() { sed -nE 's/.* beams ([0-9]+), sails.*/\1/p' <<<"$1"; }
sails_of() { sed -nE 's/.* sails ([0-9]+)\/[0-9]+.*/\1/p' <<<"$1"; }
sail_max_of() { sed -nE 's/.* sails [0-9]+\/([0-9]+).*/\1/p' <<<"$1"; }
progress_of() { sed -nE 's/.* progress ([0-9]+)%.*/\1/p' <<<"$1"; }
structure_of() { sed -nE 's/.* structure ([0-9]+)\/16.*/\1/p' <<<"$1"; }
down_of() { sed -nE 's/.* down ([0-9]+), t=.*/\1/p' <<<"$1"; }
tick_of() { sed -nE 's/.* t=([0-9]+).*/\1/p' <<<"$1"; }

expect_status() { # expect_status SUBSTRING
    local line
    line=$(status "$X" "$Y" "$Z")
    # The progress line goes to stderr: the caller captures stdout, and a second
    # copy of the fields would make every `*_of` helper answer twice.
    echo "  status: $line" >&2
    case "$line" in
        *"$1"*) ;;
        *) fail "expected /dysoncubeproject status to say '$1', got '$line'" ;;
    esac
    echo "$line"
}

# The control that makes the formation legs fail-capable: a launcher-built game
# **without** this package. Neither the controller item nor either block id may
# exist there, so the "16 placeholders" and "the controller is there" assertions
# cannot be met by a game that merely has the blocks from another source.
control_leg() {
    echo "== control: a launcher-built game without this package =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    arena
    ctl give dysoncubeproject:ray_receiver_controller 1 >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "dysoncubeproject:ray_receiver_controller")] | length == 0' >/dev/null \
        || fail "the control game has the controller item, so it is not a game without this package"
    ctl command "execute in minecraft:overworld run setblock $X $Y $Z dysoncubeproject:ray_receiver_controller" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    local name multiline
    name=$(block_name "$X" "$Y" "$Z")
    multiline=$(block_name "$X" "$((Y + 2))" "$Z")
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    [ "$multiline" = "minecraft:air" ] || fail "the control game holds $multiline"
    echo "  without this package: no controller item, and the block id stayed $name"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (Overworld sky arena, launcher-built game with this package) =="
if [ -n "$CONTROL_GAME" ]; then control_leg; fi
launch
arena

echo "== A: control - a block in the box makes the placement refuse =="
ctl command "execute in minecraft:overworld run setblock $BLOCKER minecraft:stone" >/dev/null
ctl wait 4 >/dev/null
place_controller
[ "$(block_name "$X" "$Y" "$Z")" = "minecraft:air" ] \
    || fail "a controller was placed into a blocked box: $(block_name "$X" "$Y" "$Z")"
[ "$(count_of dysoncubeproject:multiblock_structure $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)))" = 0 ] \
    || fail "placeholders appeared around a controller that was never placed"
echo "  with a stone at $BLOCKER: the controller stayed air and no placeholder appeared"
ctl command "execute in minecraft:overworld run setblock $BLOCKER minecraft:air" >/dev/null
ctl wait 4 >/dev/null

echo "== B: the clear box forms the multiblock =="
place_controller
[ "$(block_name "$X" "$Y" "$Z")" = "dysoncubeproject:ray_receiver_controller" ] \
    || fail "the controller was not placed: $(block_name "$X" "$Y" "$Z")"
placeholders=$(count_of dysoncubeproject:multiblock_structure $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)))
[ "$placeholders" = 16 ] || fail "expected 16 placeholders, found $placeholders"
echo "  controller at $CONTROLLER with $placeholders multiblock_structure placeholders"

echo "== C: the readout of a fresh receiver =="
line=$(expect_status "formed, box 3x6x3, structure 16/16")
[ "$(beams_of "$line")" = 0 ] || fail "a fresh sphere has beams: $line"
[ "$(sails_of "$line")" = 0 ] || fail "a fresh sphere has sails: $line"
[ "$(sail_max_of "$line")" = 0 ] || fail "a sphere with no beams supports sails: $line"
[ "$(buffer_of "$line")" = 0 ] || fail "a fresh receiver holds FE: $line"
[ "$(down_of "$line")" = 0 ] || fail "a fresh receiver pushed FE: $line"

echo "== D: upstream's set beams / set panels, with their ceilings (at night, so nothing fills) =="
ctl command "time set 18000" >/dev/null
ctl wait 20
change "set beams" "$X $Y $Z 8333334"
line=$(expect_status "beams 8333334")
[ "$(sail_max_of "$line")" = 50000000 ] || fail "8,333,334 beams must support 50,000,000 sails: $line"
change "set panels" "$X $Y $Z 100"
line=$(expect_status "sails 100/50000000")
change "add panels" "$X $Y $Z 100000000"
line=$(expect_status "sails 50000000/50000000")
echo "  beams 8,333,334 -> 50,000,000 sails, and 100,000,000 added sails clamp to the ceiling"
change "add beams" "$X $Y $Z -8333333"
line=$(expect_status "beams 1")
change "set panels" "$X $Y $Z 100000000"
line=$(expect_status "sails 6/6")
echo "  one beam supports six sails, as BEAM_TO_SOLAR_PANEL_RATIO says"
[ "$(progress_of "$line")" = 0 ] || fail "6 of 50,000,000 sails is not 0%: $line"

echo "== E: 100 sails make 2,000 FE/t =="
ctl command "time set 1000" >/dev/null
ctl wait 20
change "set beams" "$X $Y $Z 17"
change "set panels" "$X $Y $Z 100"
line=$(expect_status "sails 100/102")
[ "$(gen_of "$line")" = 2000 ] || fail "100 sails must generate 2,000 FE/t: $line"
start_buffer=$(buffer_of "$line")
start_tick=$(tick_of "$line")
ctl wait 100
line=$(status "$X" "$Y" "$Z")
echo "  $line"
end_buffer=$(buffer_of "$line")
end_tick=$(tick_of "$line")
elapsed=$((end_tick - start_tick))
rate=$(awk -v a="$start_buffer" -v b="$end_buffer" -v d="$elapsed" 'BEGIN { printf "%.1f", (b - a) / d }')
echo "  $elapsed ticks: $start_buffer -> $end_buffer FE, $rate FE/t (upstream 2,000)"
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting a wildly wrong rate (this must fail)"
    rate=1.0
fi
awk -v rate="$rate" 'BEGIN { exit !(rate > 1990 && rate < 2010) }' \
    || fail "the receiver filled at $rate FE/t, not 100 sails * 20: $line"

echo "== E2: control - a stone roof over the machine stops the harvest =="
# The machine's own shell is skipped (upstream's literal check never opens, see
# notes.md); a block that is *not* its shell must still shadow it, or the gate
# would be vacuous.
ctl command "execute in minecraft:overworld run setblock $X $((Y + 5)) $Z minecraft:stone" >/dev/null
ctl wait 20
roof_start=$(buffer_of "$(status "$X" "$Y" "$Z")")
ctl wait 60
line=$(status "$X" "$Y" "$Z")
roof_end=$(buffer_of "$line")
echo "  under a stone roof at $X $((Y + 5)) $Z: $roof_start -> $roof_end FE over 60 ticks"
[ "$roof_end" = "$roof_start" ] || fail "the receiver harvested under a stone roof: $line"
ctl command "execute in minecraft:overworld run setblock $X $((Y + 5)) $Z minecraft:air" >/dev/null
ctl wait 20
roof_off_start=$(buffer_of "$(status "$X" "$Y" "$Z")")
ctl wait 60
line=$(status "$X" "$Y" "$Z")
roof_off_end=$(buffer_of "$line")
echo "  roof removed: $roof_off_start -> $roof_off_end FE over 60 ticks"
[ "$roof_off_end" -gt "$roof_off_start" ] || fail "the receiver did not resume without the roof: $line"

echo "== F: control - night stops the harvest, day starts it again =="
ctl command "time set 18000" >/dev/null
ctl wait 20
night_start=$(buffer_of "$(status "$X" "$Y" "$Z")")
ctl wait 60
line=$(status "$X" "$Y" "$Z")
night_end=$(buffer_of "$line")
echo "  night: $night_start -> $night_end FE over 60 ticks"
[ "$night_end" = "$night_start" ] || fail "the receiver harvested at night: $line"
ctl command "time set 1000" >/dev/null
ctl wait 20
day_start=$(buffer_of "$(status "$X" "$Y" "$Z")")
ctl wait 60
line=$(status "$X" "$Y" "$Z")
day_end=$(buffer_of "$line")
echo "  day again: $day_start -> $day_end FE over 60 ticks"
[ "$day_end" -gt "$day_start" ] || fail "the receiver did not resume by day: $line"

echo "== G: the caps - 1,000,000,000 FE/t into a 100,000,000 FE buffer =="
change "set beams" "$X $Y $Z 8333334"
change "set panels" "$X $Y $Z 50000000"
line=$(expect_status "gen 1000000000 FE/t")
ctl wait 20
line=$(status "$X" "$Y" "$Z")
echo "  $line"
[ "$(buffer_of "$line")" = 100000000 ] || fail "the buffer did not stop at its 100,000,000 FE capacity: $line"
[ "$(down_of "$line")" = 0 ] || fail "FE left the receiver with no sink below: $line"
echo "  the buffer is full at 100,000,000 FE and nothing left the machine (no FE sink below it)"

echo "== H: a broken placeholder takes the controller with it =="
ctl look-at "$((X + 1))" "$Y" "$Z" >/dev/null
ctl break "$((X + 1))" "$Y" "$Z" >/dev/null 2>&1 || true
ctl wait 6 >/dev/null
[ "$(block_name "$X" "$Y" "$Z")" = "minecraft:air" ] \
    || fail "breaking a placeholder left the controller: $(block_name "$X" "$Y" "$Z")"
left=$(count_of dysoncubeproject:multiblock_structure $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)))
echo "  after breaking one placeholder: the controller is gone, $left placeholders left"

echo "== I: a broken controller clears its placeholders =="
ctl command "execute in minecraft:overworld run fill $((X - 1)) $Y $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)) minecraft:air" >/dev/null
ctl wait 6 >/dev/null
place_controller
rebuilt=$(block_name "$X" "$Y" "$Z")
if [ "$rebuilt" != "dysoncubeproject:ray_receiver_controller" ]; then
    echo "  volume: $(ctl blocks $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)) | jq -c .counts)"
    echo "  hotbar: $(ctl inventory | jq -c '[.hotbar[] | select(. != null) | .item]')"
    echo "  state: $(ctl state | jq -c '{held, selected, target, dimension}')"
    fail "the controller was not placed again: $rebuilt"
fi
before_placeholders=$(count_of dysoncubeproject:multiblock_structure $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)))
[ "$before_placeholders" = 16 ] || fail "the rebuilt multiblock has $before_placeholders placeholders"
# The shell stands between the player and the controller; clear the one facing
# them (not with a player's break, which would take the controller with it) so
# the controller itself can be mined.
ctl command "execute in minecraft:overworld run setblock $((X + 1)) $Y $Z minecraft:air" >/dev/null
ctl wait 4 >/dev/null
reachable=$(count_of dysoncubeproject:multiblock_structure $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)))
echo "  with the near placeholder cleared: $reachable placeholders, the controller is reachable"
ctl look-at "$X" "$Y" "$Z" >/dev/null
ctl break "$X" "$Y" "$Z" >/dev/null 2>&1 || true
ctl wait 6 >/dev/null
[ "$(block_name "$X" "$Y" "$Z")" = "minecraft:air" ] || fail "the controller did not break"
after_placeholders=$(count_of dysoncubeproject:multiblock_structure $((X - 1)) "$Y" $((Z - 1)) $((X + 1)) $((Y + 5)) $((Z + 1)))
[ "$after_placeholders" = 0 ] || fail "the broken controller left $after_placeholders placeholders"
echo "  after breaking the controller: 0 placeholders left"

echo "PASS: the 3x6x3 box refused a blocked placement and formed when clear (16 placeholders), the readout showed the sphere, set/add beams and panels clamped to their ceilings, 100 sails filled the buffer at 2,000 FE/t, a stone roof stopped the harvest while the machine's own shell did not, night stopped it and day resumed it, 50,000,000 sails generated 1,000,000,000 FE/t into a 100,000,000 FE buffer that held and pushed nothing with no sink, a broken placeholder took the controller with it, and a broken controller cleared its placeholders"
