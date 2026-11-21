#!/usr/bin/env bash
# In-game check for Dyson Cube Project slices 1–2: the ray receiver, its multiblock
# and the shared per-player sphere.
#
# **The game must be a launcher build of this package** (mod API 0.2):
#
#   ports/tools/launcher-game.sh build dysoncubeproject --port ports/dysoncubeproject
#   eval "$(ports/tools/launcher-game.sh env dysoncubeproject)"
#   tools/e2e.sh
#
# There is no `--dev-mod` any more: a native `.hmod` carries its code *and* its
# content, and the launcher compiles it against the pinned build. Every mod command
# is typed with `ctl chat "/cmd"` because `ctl command` runs the console path and
# bypasses the native command hook.
#
# The Overworld (the only level whose tick generates sphere power upstream), high over
# the terrain: a stone platform at y=179 from x=90 to x=116, receiver A at (100,180,0)
# and receiver B at (110,180,0). `time set 1000` and `weather clear` pin the sky.
# The launch is `--offscreen` because the crafting phase opens a menu (a headless
# session has no renderer and `ctl screen` cannot read one).
#
# Default run:
#  C0 the five carried recipes craft in a crafting table, each the way it declares
#     (output id and count, and the ingredients consumed): beam x2, solar_sail,
#     beam_package, solar_sail_package, ray_receiver_controller. The bulk ingredients
#     for the package/controller crafts are given, so each recipe is exercised once;
#  A  control "wrong shape": a stone inside the 3x6x3 box refuses the placement — the
#     block stays air and no placeholder appears;
#  B  the same placement with a clear box forms: the controller and sixteen
#     `multiblock_structure` placeholders;
#  C  the readout: formed, 16/16, the placer's own UUID as the sphere id, an empty
#     sphere (beams 0, sails 0/0), no FE;
#  D  `/dysoncubeproject set|add beams|panels <sphere> <n>`: the sail ceiling follows
#     the beams (8,333,334 beams -> 50,000,000 sails), add clamps at both ends;
#  E  one receiver: 100 sails make 2,000 FE/t into its buffer, measured over `t=`;
#  F  TWO receivers on one sphere (the same id): the pair's buffers grow at 2,000 FE/t
#     *together* — one shared reservoir, not one per receiver (two independent spheres
#     would give 4,000 FE/t);
#  F2 a stone roof over A: A stops, B alone takes the 2,000 FE/t; removing the roof
#     resumes A — the same shared sphere, gated per receiver by its own sky;
#  G  a second sphere id is independent: setting its panels leaves the receivers' sails
#     where they were;
#  H  control "night": B stops filling at t=18000 and resumes at t=1000;
#  I  the caps: 50,000,000 sails generate 1,000,000,000 FE/t, B's buffer stops at
#     100,000,000 FE and nothing leaves (no FE sink below);
#  J  persistence: `save` + quit + relaunch the same world keeps the sphere (its id,
#     beams and sails) and B's buffer — the level's `data/horizon/attachments.dat`;
#  K  a broken placeholder takes the controller with it; a broken controller clears
#     its placeholders.
#
# CONTROL_GAME=<a launcher build without this package> adds an in-run control leg (the
# ids are unknown there). Running the whole scenario with GAME=$CONTROL_GAME must FAIL:
# that run is the control log.
#
# SELFTEST=1 flips E's rate assertion (it must FAIL), proving the scenario can fail.
set -Eeuo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/dysoncubeproject
# The launcher-built game carrying this package. Required.
GAME=${GAME:-}
# A launcher-built game *without* this package, for the in-run control leg (optional).
CONTROL_GAME=${CONTROL_GAME:-}
# The control client ships beside the launcher-built game; override with CTL=.
CTL=${CTL:-$(dirname "${GAME:-/nonexistent}" 2>/dev/null)/horizon-ctl}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-${E2E_TMP:-/tmp/opencode}/dysoncubeproject-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=DysonCubeProjectE2E
CONTROL_WORLD=DysonCubeProjectNoMod

SPHERE_CMD=dysoncubeproject
CONTROLLER_ITEM=dysoncubeproject:ray_receiver_controller
SHELL=dysoncubeproject:multiblock_structure

[ -n "$GAME" ] || {
    echo "missing GAME: this package needs a launcher-built game (see the header)" >&2
    exit 2
}
[ -x "$GAME" ] || { echo "missing launcher-built horizon-client: $GAME" >&2; exit 1; }
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL (set CTL=)" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

# The floor and the two receivers in the Overworld's sky: X,Y,Z is the controller's cell.
FLOOR_Y=179
AX=100
BX=110
Y=180
AZ=0
BZ=0
# A cell inside A's 3x6x3 box (x-1..x+1, y..y+5, z-1..z+1) that is not a placeholder.
BLOCKER="$AX $((Y + 3)) $AZ"
# The crafting table, on the same platform.
CRAFT_X=95
CRAFT_Z=-3

# `/tmp/opencode` is the campaign's scratch root; the second machine starts without it.
mkdir -p "${E2E_TMP:-/tmp/opencode}"
SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/dysoncubeproject-home.XXXXXX)
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
        --creative --offscreen --wait --resources "$RES" >/dev/null
    ctl wait-ready 180 >/dev/null
    ctl wait 40 >/dev/null
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty'; }
count_of() { # count_of BLOCK X1 Y1 Z1 X2 Y2 Z2
    ctl blocks "$2" "$3" "$4" "$5" "$6" "$7" | jq -r --arg b "$1" '.counts[$b] // 0'
}

# A stone platform at y=179 in the Overworld's sky, clear air above it.
arena() {
    ctl command "execute in minecraft:overworld run tp @s $((AX + 2)).5 $Y $((AZ + 2)).5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$AX" "$FLOOR_Y" "$AZ" | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the arena chunk never loaded"
    ctl command "execute in minecraft:overworld run fill 88 $FLOOR_Y -8 118 $FLOOR_Y 8 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:overworld run fill 88 $Y -8 118 $((Y + 10)) 8 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:overworld run kill @e[type=!player]" >/dev/null
    ctl command "time set 1000" >/dev/null
    ctl command "weather clear 1000000" >/dev/null
    ctl command "execute in minecraft:overworld run tp @s $((AX + 2)).5 $Y $((AZ + 2)).5" >/dev/null
    ctl wait 20
    [ "$(block_name "$AX" "$FLOOR_Y" "$AZ")" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$AX" "$Y" "$AZ")" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# --- the crafting table -----------------------------------------------------------------
# The 3x3 grid is slots 100-108 (row by row), the result 200, and the hotbar 0-8.
# A left click picks a whole stack up, a right click drops one item of it into a cell.
hotbar() { # hotbar SLOT ITEM COUNT
    ctl command "item replace entity @s hotbar.$1 with $2 $3" >/dev/null
}
stand_at() { ctl command "execute in minecraft:overworld run tp @s $1.5 $2 $3.5" >/dev/null; ctl wait 3; }

# craft NAME RESULT COUNT H0 H1 H2 H3 H4 H5 H6 H7 H8
# Each Hn is the hotbar slot whose stack feeds that grid cell, or "-" for an empty cell.
craft() {
    local name=$1 result=$2 count=$3
    shift 3
    local grid=(100 101 102 103 104 105 106 107 108)
    local cells=("$@")
    ctl interact "$CRAFT_X" "$Y" "$CRAFT_Z" >/dev/null || fail "the crafting table did not open"
    ctl wait-for screen inventory --timeout 15 >/dev/null \
        || fail "the crafting table did not open a menu (screen=$(ctl state | jq -r '.screen // ""'))"
    local i=0 hs cell
    while [ "$i" -lt 9 ]; do
        hs=${cells[$i]:--}
        cell=${grid[$i]}
        if [ "$hs" != "-" ]; then
            ctl click-slot "$hs" >/dev/null; ctl wait 2
            ctl click-slot "$cell" right >/dev/null; ctl wait 2
            ctl click-slot "$hs" >/dev/null; ctl wait 2
        fi
        i=$((i + 1))
    done
    local result_slot
    result_slot=$(ctl screen | jq -c '[.slots[] | select(.slot == 200)][0] // {}')
    echo "  $name result slot: $result_slot"
    jq -e --arg id "$result" --argjson n "$count" \
        '.item == $id and .count == $n' <<<"$result_slot" >/dev/null \
        || fail "the $name recipe offered $result_slot, expected $count x $result"
    ctl click-slot 200 >/dev/null || fail "cannot take the crafted $name"
    ctl wait 3
    ctl click-slot 8 >/dev/null || fail "cannot store the crafted $name in the hotbar"
    ctl wait 3
    local left
    left=$(ctl screen | jq -c '[.slots[] | select(.slot >= 100 and .slot <= 108 and .item != null)]')
    [ "$left" = "[]" ] || fail "the $name grid still holds $left after the craft"
    echo "  $name: $count x $result, every ingredient consumed"
    ctl close >/dev/null 2>&1 || true
    ctl wait 4
}

craft_phase() {
    echo "== C0: the five carried recipes craft in a crafting table =="
    ctl command "execute in minecraft:overworld run setblock $CRAFT_X $Y $CRAFT_Z minecraft:crafting_table" >/dev/null
    ctl wait 4
    stand_at "$((CRAFT_X + 2))" "$Y" "$CRAFT_Z"
    # beam: NIN / BIB / NIN  (N = iron nugget, I = iron block, B = iron bars) -> 2 beams
    hotbar 0 minecraft:iron_nugget 8
    hotbar 1 minecraft:iron_block 8
    hotbar 2 minecraft:iron_bars 8
    ctl wait 6
    craft "beam" dysoncubeproject:beam 2 0 1 0 2 1 2 0 1 0
    # solar_sail: GCG / GCG / LCL  (G = glass pane, C = copper ingot, L = lapis) -> 1
    hotbar 0 minecraft:glass_pane 8
    hotbar 1 minecraft:copper_ingot 8
    hotbar 2 minecraft:lapis_lazuli 8
    ctl wait 6
    craft "solar_sail" dysoncubeproject:solar_sail 1 0 1 0 0 1 0 2 1 2
    # beam_package: " G " / "GIG" / " G "  (G = beam, I = copper block) -> 1
    hotbar 0 dysoncubeproject:beam 4
    hotbar 1 minecraft:copper_block 4
    ctl wait 6
    craft "beam_package" dysoncubeproject:beam_package 1 - 0 - 0 1 0 - 0 -
    # solar_sail_package: GGG / GIG / GGG  (G = solar sail, I = iron block) -> 1
    hotbar 0 dysoncubeproject:solar_sail 8
    hotbar 1 minecraft:iron_block 4
    ctl wait 6
    craft "solar_sail_package" dysoncubeproject:solar_sail_package 1 0 0 0 0 1 0 0 0 0
    # ray_receiver_controller: SSS / NBN / III  (S = sail, N = smooth stone slab, B = beam)
    hotbar 0 dysoncubeproject:solar_sail 3
    hotbar 1 minecraft:smooth_stone_slab 3
    hotbar 2 dysoncubeproject:beam 2
    hotbar 3 minecraft:iron_block 3
    ctl wait 6
    craft "ray_receiver_controller" dysoncubeproject:ray_receiver_controller 1 0 0 0 1 2 1 3 3 3
}

# --- the receiver --------------------------------------------------------------------
give_controller() { # give_controller: one controller item in the held hotbar slot
    ctl command "item replace entity @s hotbar.8 with $CONTROLLER_ITEM 1" >/dev/null
    ctl wait 4
}

place_controller() { # place_controller X Z: place the held controller
    ctl select "$CONTROLLER_ITEM" >/dev/null || fail "the hotbar holds no controller item"
    ctl wait 2
    ctl place "$1" "$Y" "$2" >/dev/null 2>&1 || true
    ctl wait 8
}

# status X Y Z: this controller's status line, after a fresh reply arrives (the `t=`
# field makes each reply unique).
status() { # status X Y Z
    local out line before
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E "Ray Receiver at $1,$2,$3:|No ray receiver" | head -1)
    say "$SPHERE_CMD status $1 $2 $3"
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

# change FORM: send one of upstream's `set|add beams|panels <sphere> <n>` forms.
change() { # change FORM SPHERE VALUE
    say "$SPHERE_CMD $1 $2 $3"
    ctl wait 4 >/dev/null
}

buffer_of() { sed -nE 's/.* buffer ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
gen_of() { sed -nE 's/.* gen ([0-9]+) FE\/t.*/\1/p' <<<"$1"; }
con_of() { sed -nE 's/.* con ([0-9]+) FE\/t.*/\1/p' <<<"$1"; }
beams_of() { sed -nE 's/.* beams ([0-9]+), sails.*/\1/p' <<<"$1"; }
sails_of() { sed -nE 's/.* sails ([0-9]+)\/[0-9]+.*/\1/p' <<<"$1"; }
sail_max_of() { sed -nE 's/.* sails [0-9]+\/([0-9]+).*/\1/p' <<<"$1"; }
progress_of() { sed -nE 's/.* progress ([0-9]+)%.*/\1/p' <<<"$1"; }
sphere_of() { sed -nE "s/.* sphere '([^']*)'.*/\1/p" <<<"$1"; }
structure_of() { sed -nE 's/.* structure ([0-9]+)\/16.*/\1/p' <<<"$1"; }
down_of() { sed -nE 's/.* down ([0-9]+), t=.*/\1/p' <<<"$1"; }
tick_of() { sed -nE 's/.* t=([0-9]+).*/\1/p' <<<"$1"; }

expect_status() { # expect_status X Y Z SUBSTRING
    local line
    line=$(status "$1" "$2" "$3")
    # The progress line goes to stderr: the caller captures stdout, and a second
    # copy of the fields would make every `*_of` helper answer twice.
    echo "  status($1,$2,$3): $line" >&2
    case "$line" in
        *"$4"*) ;;
        *) fail "expected /dysoncubeproject status at $1,$2,$3 to say '$4', got '$line'" ;;
    esac
    echo "$line"
}

# The in-run control: a launcher-built game **without** this package. Neither the
# controller item nor the block ids may exist there, so the formation assertions cannot
# be met by a game that merely holds the blocks from another source.
control_leg() {
    echo "== control: a launcher-built game without this package =="
    launch "$CONTROL_WORLD" "$CONTROL_GAME"
    arena
    ctl command "item replace entity @s hotbar.8 with $CONTROLLER_ITEM 1" >/dev/null 2>&1 || true
    ctl wait 6 >/dev/null
    ctl inventory | jq -e --arg i "$CONTROLLER_ITEM" \
        '[.hotbar[], .main[] | select(.item == $i)] | length == 0' >/dev/null \
        || fail "the control game has the controller item, so it is not a game without this package"
    ctl command "execute in minecraft:overworld run setblock $AX $Y $AZ $CONTROLLER_ITEM" >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
    local name multiline
    name=$(block_name "$AX" "$Y" "$AZ")
    multiline=$(block_name "$AX" "$((Y + 2))" "$AZ")
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    [ "$multiline" = "minecraft:air" ] || fail "the control game holds $multiline"
    echo "  without this package: no controller item, and the block id stayed $name"
    ctl quit >/dev/null 2>&1 || true
    sleep 2
}

echo "== launch (Overworld sky arena, launcher-built game with this package) =="
if [ -n "$CONTROL_GAME" ]; then control_leg; fi
launch
arena
craft_phase

echo "== A: control - a block in the box makes the placement refuse =="
ctl command "execute in minecraft:overworld run setblock $BLOCKER minecraft:stone" >/dev/null
ctl wait 4 >/dev/null
stand_at "$((AX + 2))" "$Y" "$((AZ + 2))"
place_controller "$AX" "$AZ"
[ "$(block_name "$AX" "$Y" "$AZ")" = "minecraft:air" ] \
    || fail "a controller was placed into a blocked box: $(block_name "$AX" "$Y" "$AZ")"
[ "$(count_of $SHELL $((AX - 1)) "$Y" $((AZ - 1)) $((AX + 1)) $((Y + 5)) $((AZ + 1)))" = 0 ] \
    || fail "placeholders appeared around a controller that was never placed"
echo "  with a stone at $BLOCKER: the controller stayed air and no placeholder appeared"
ctl command "execute in minecraft:overworld run setblock $BLOCKER minecraft:air" >/dev/null
ctl wait 4 >/dev/null

echo "== B: the clear box forms the multiblock =="
# The controller placed here is the one crafted in phase C0: the recipe's own product
# is what forms the multiblock.
place_controller "$AX" "$AZ"
[ "$(block_name "$AX" "$Y" "$AZ")" = "$CONTROLLER_ITEM" ] \
    || fail "the crafted controller was not placed: $(block_name "$AX" "$Y" "$AZ")"
placeholders=$(count_of $SHELL $((AX - 1)) "$Y" $((AZ - 1)) $((AX + 1)) $((Y + 5)) $((AZ + 1)))
[ "$placeholders" = 16 ] || fail "expected 16 placeholders, found $placeholders"
echo "  the crafted controller at $AX $Y $AZ formed $placeholders $SHELL placeholders"

echo "== C: the readout of a fresh receiver names its owner's sphere =="
line=$(expect_status "$AX" "$Y" "$AZ" "formed, box 3x6x3, structure 16/16")
SPHERE=$(sphere_of "$line")
echo "  the sphere id is '$SPHERE'"
printf '%s' "$SPHERE" | grep -Eq '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$' \
    || fail "the sphere id '$SPHERE' is not a UUID, so it is not the placer's"
[ "$(beams_of "$line")" = 0 ] || fail "a fresh sphere has beams: $line"
[ "$(sails_of "$line")" = 0 ] || fail "a fresh sphere has sails: $line"
[ "$(sail_max_of "$line")" = 0 ] || fail "a sphere with no beams supports sails: $line"
[ "$(buffer_of "$line")" = 0 ] || fail "a fresh receiver holds FE: $line"
[ "$(down_of "$line")" = 0 ] || fail "a fresh receiver pushed FE: $line"

echo "== D: upstream's set beams / set panels, with their ceilings (at night, so nothing fills) =="
ctl command "time set 18000" >/dev/null
ctl wait 20
change "set beams" "$SPHERE" 8333334
line=$(expect_status "$AX" "$Y" "$AZ" "beams 8333334")
[ "$(sail_max_of "$line")" = 50000000 ] || fail "8,333,334 beams must support 50,000,000 sails: $line"
change "set panels" "$SPHERE" 100
line=$(expect_status "$AX" "$Y" "$AZ" "sails 100/50000000")
change "add panels" "$SPHERE" 100000000
line=$(expect_status "$AX" "$Y" "$AZ" "sails 50000000/50000000")
echo "  beams 8,333,334 -> 50,000,000 sails, and 100,000,000 added sails clamp to the ceiling"
change "add beams" "$SPHERE" -8333333
line=$(expect_status "$AX" "$Y" "$AZ" "beams 1")
change "set panels" "$SPHERE" 100000000
line=$(expect_status "$AX" "$Y" "$AZ" "sails 6/6")
echo "  one beam supports six sails, as BEAM_TO_SOLAR_PANEL_RATIO says"
[ "$(progress_of "$line")" = 0 ] || fail "6 of 50,000,000 sails is not 0%: $line"

echo "== E: 100 sails make 2,000 FE/t at one receiver =="
ctl command "time set 1000" >/dev/null
ctl wait 20
change "set beams" "$SPHERE" 17
change "set panels" "$SPHERE" 100
line=$(expect_status "$AX" "$Y" "$AZ" "sails 100/102")
[ "$(gen_of "$line")" = 2000 ] || fail "100 sails must generate 2,000 FE/t: $line"
start_buffer=$(buffer_of "$line")
start_tick=$(tick_of "$line")
ctl wait 100
line=$(status "$AX" "$Y" "$AZ")
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

echo "== F: a second receiver joins the same sphere (one shared reservoir) =="
stand_at "$((BX + 2))" "$Y" "$((BZ + 2))"
give_controller
place_controller "$BX" "$BZ"
[ "$(block_name "$BX" "$Y" "$BZ")" = "$CONTROLLER_ITEM" ] \
    || fail "the second controller was not placed: $(block_name "$BX" "$Y" "$BZ")"
b_placeholders=$(count_of $SHELL $((BX - 1)) "$Y" $((BZ - 1)) $((BX + 1)) $((Y + 5)) $((BZ + 1)))
[ "$b_placeholders" = 16 ] || fail "the second multiblock has $b_placeholders placeholders"
line=$(expect_status "$BX" "$Y" "$BZ" "sails 100/102")
b_sphere=$(sphere_of "$line")
[ "$b_sphere" = "$SPHERE" ] \
    || fail "the second receiver subscribed to '$b_sphere', not the placer's '$SPHERE'"
echo "  the second receiver reads the same sphere '$SPHERE' (sails 100/102)"

# The pair's two buffers together must grow at one sphere's rate: 2,000 FE/t. Two
# independent per-receiver spheres would give 4,000 FE/t. Order-independent, because
# whichever receiver ticks first takes the tick's reservoir.
pair() { # pair -> "a b t"
    local a b
    a=$(status "$AX" "$Y" "$AZ")
    b=$(status "$BX" "$Y" "$BZ")
    echo "$(buffer_of "$a") $(buffer_of "$b") $(tick_of "$a")"
}
read -r a1 b1 t1 <<<"$(pair)"
ctl wait 100
read -r a2 b2 t2 <<<"$(pair)"
elapsed=$((t2 - t1))
sum_rate=$(awk -v a1="$a1" -v b1="$b1" -v a2="$a2" -v b2="$b2" -v d="$elapsed" \
    'BEGIN { printf "%.1f", ((a2 + b2) - (a1 + b1)) / d }')
echo "  $elapsed ticks: A $a1 -> $a2, B $b1 -> $b2, together $sum_rate FE/t (one sphere: 2,000)"
awk -v rate="$sum_rate" 'BEGIN { exit !(rate > 1990 && rate < 2010) }' \
    || fail "the two receivers filled at $sum_rate FE/t together, not one sphere's 2,000: A $a1->$a2, B $b1->$b2"

echo "== F2: control - the roof over A moves the shared power to B =="
ctl command "execute in minecraft:overworld run setblock $AX $((Y + 5)) $AZ minecraft:stone" >/dev/null
ctl wait 20
read -r a1 b1 t1 <<<"$(pair)"
ctl wait 60
read -r a2 b2 t2 <<<"$(pair)"
echo "  under A's roof over 60 ticks: A $a1 -> $a2, B $b1 -> $b2"
[ "$a2" = "$a1" ] || fail "A harvested under a stone roof: A $a1 -> $a2"
[ "$b2" -gt "$b1" ] || fail "B did not take the shared power while A was roofed: B $b1 -> $b2"
b_rate=$(awk -v a="$b1" -v b="$b2" -v d="$((t2 - t1))" 'BEGIN { printf "%.1f", (b - a) / d }')
echo "  B alone: $b_rate FE/t"
awk -v rate="$b_rate" 'BEGIN { exit !(rate > 1990 && rate < 2010) }' \
    || fail "B alone filled at $b_rate FE/t, not 2,000"
ctl command "execute in minecraft:overworld run setblock $AX $((Y + 5)) $AZ minecraft:air" >/dev/null
ctl wait 20
read -r a3 b3 t3 <<<"$(pair)"
ctl wait 40
read -r a4 b4 t4 <<<"$(pair)"
echo "  roof removed over 40 ticks: A $a3 -> $a4, B $b3 -> $b4"
[ "$a4" -gt "$a3" ] || fail "A did not resume without the roof: A $a3 -> $a4"

echo "== G: a second sphere id is independent of the receivers' =="
OTHER="keepalive-$(printf '%s' "$SPHERE" | cut -c1-8)"
change "set beams" "$OTHER" 10
change "set panels" "$OTHER" 60
line=$(expect_status "$BX" "$Y" "$BZ" "sails 100/102")
[ "$(sphere_of "$line")" = "$SPHERE" ] || fail "the second receiver changed spheres: $line"
[ "$(beams_of "$line")" = 17 ] || fail "the other sphere's beams reached the receivers: $line"
echo "  sphere '$OTHER' holds 10 beams / 60 sails; the receivers still read 17 beams / 100 sails"

echo "== H: control - night stops the harvest, day starts it again =="
# The pair is the unit: one sphere's reservoir, so the sum of the two buffers is what
# night stops and day resumes (the tick's power goes to whichever receiver takes it).
ctl command "time set 18000" >/dev/null
ctl wait 20
read -r a1 b1 t1 <<<"$(pair)"
ctl wait 60
read -r a2 b2 t2 <<<"$(pair)"
echo "  night over $((t2 - t1)) ticks: A $a1 -> $a2, B $b1 -> $b2"
[ "$((a2 + b2))" = "$((a1 + b1))" ] || fail "the pair harvested at night: A $a1 -> $a2, B $b1 -> $b2"
ctl command "time set 1000" >/dev/null
ctl wait 20
read -r a3 b3 t3 <<<"$(pair)"
ctl wait 60
read -r a4 b4 t4 <<<"$(pair)"
day_rate=$(awk -v a="$((a3 + b3))" -v b="$((a4 + b4))" -v d="$((t4 - t3))" 'BEGIN { printf "%.1f", (b - a) / d }')
echo "  day again over $((t4 - t3)) ticks: A $a3 -> $a4, B $b3 -> $b4, together $day_rate FE/t"
[ "$((a4 + b4))" -gt "$((a3 + b3))" ] || fail "the pair did not resume by day: A $a3 -> $a4, B $b3 -> $b4"
awk -v rate="$day_rate" 'BEGIN { exit !(rate > 1990 && rate < 2010) }' \
    || fail "the pair resumed at $day_rate FE/t, not 2,000"

echo "== I: the caps - 1,000,000,000 FE/t into a 100,000,000 FE buffer =="
change "set beams" "$SPHERE" 8333334
change "set panels" "$SPHERE" 50000000
line=$(expect_status "$BX" "$Y" "$BZ" "gen 1000000000 FE/t")
ctl wait 20
line=$(status "$BX" "$Y" "$BZ")
echo "  $line"
[ "$(buffer_of "$line")" = 100000000 ] || fail "the buffer did not stop at its 100,000,000 FE capacity: $line"
[ "$(down_of "$line")" = 0 ] || fail "FE left the receiver with no sink below: $line"
echo "  the buffer is full at 100,000,000 FE and nothing left the machine (no FE sink below it)"

echo "== J: the sphere survives a save, a quit and a relaunch =="
line=$(status "$BX" "$Y" "$BZ")
before_sails=$(sails_of "$line")
before_beams=$(beams_of "$line")
before_buffer=$(buffer_of "$line")
echo "  before: sphere '$SPHERE', beams $before_beams, sails $before_sails, buffer $before_buffer"
ctl save >/dev/null
ctl wait 20 >/dev/null
ctl quit >/dev/null 2>&1 || true
sleep 3
STORE="$SCRATCH/saves/$WORLD/dimensions/minecraft/overworld/data/horizon/attachments.dat"
[ -s "$STORE" ] || fail "the level kept no attachments file at $STORE"
gunzip -c "$STORE" 2>/dev/null | grep -a -q "dysoncubeproject" \
    || fail "the attachments file does not mention this mod: $(gunzip -c "$STORE" 2>/dev/null | head -c 200 | tr -d '\0')"
gunzip -c "$STORE" 2>/dev/null | grep -a -q "$SPHERE" \
    || fail "the attachments file does not hold the sphere '$SPHERE'"
echo "  $STORE exists and holds sphere '$SPHERE'"
launch
line=$(expect_status "$BX" "$Y" "$BZ" "sails $before_sails/")
[ "$(sphere_of "$line")" = "$SPHERE" ] || fail "the relaunched receiver reads a different sphere: $line"
[ "$(beams_of "$line")" = "$before_beams" ] || fail "the beams changed across the relaunch: $line"
after_buffer=$(buffer_of "$line")
echo "  after: sphere '$(sphere_of "$line")', beams $(beams_of "$line"), sails $(sails_of "$line"), buffer $after_buffer"
[ "$after_buffer" -ge "$before_buffer" ] || fail "the buffer regressed across the relaunch: $before_buffer -> $after_buffer"

echo "== K: a broken placeholder takes the controller with it =="
stand_at "$((BX + 2))" "$Y" "$((BZ + 2))"
# The placeholder above the controller is reachable from outside the shell; breaking it
# must take the controller (MultiblockStructureBlock.onRemove).
ctl look-at "$BX" "$((Y + 1))" "$BZ" >/dev/null
ctl break "$BX" "$((Y + 1))" "$BZ" >/dev/null 2>&1 || true
ctl wait 6 >/dev/null
[ "$(block_name "$BX" "$Y" "$BZ")" = "minecraft:air" ] \
    || fail "breaking a placeholder left the controller: $(block_name "$BX" "$Y" "$BZ")"
left=$(count_of $SHELL $((BX - 1)) "$Y" $((BZ - 1)) $((BX + 1)) $((Y + 5)) $((BZ + 1)))
echo "  after breaking a placeholder: the controller is gone, $left placeholders left"

echo "== K2: a broken controller clears its placeholders =="
stand_at "$((AX + 2))" "$Y" "$((AZ + 2))"
ctl command "execute in minecraft:overworld run setblock $((AX + 1)) $Y $AZ minecraft:air" >/dev/null
ctl wait 4 >/dev/null
ctl look-at "$AX" "$Y" "$AZ" >/dev/null
ctl break "$AX" "$Y" "$AZ" >/dev/null 2>&1 || true
ctl wait 6 >/dev/null
[ "$(block_name "$AX" "$Y" "$AZ")" = "minecraft:air" ] || fail "the controller did not break"
after_placeholders=$(count_of $SHELL $((AX - 1)) "$Y" $((AZ - 1)) $((AX + 1)) $((Y + 5)) $((AZ + 1)))
[ "$after_placeholders" = 0 ] || fail "the broken controller left $after_placeholders placeholders"
echo "  after breaking the controller: 0 placeholders left"

echo "PASS: the five carried recipes crafted in a table (output and consumption), the 3x6x3 box refused a blocked placement and formed when clear (16 placeholders), a receiver subscribed to its placer's UUID and set|add beams|panels clamped by sphere id, 100 sails filled at 2,000 FE/t, two receivers shared one reservoir at 2,000 FE/t together and the roof over one moved that power to the other, a second sphere stayed independent, night stopped the harvest and day resumed it, 50,000,000 sails generated 1,000,000,000 FE/t into a 100,000,000 FE buffer that held and pushed nothing, the level's attachments.dat kept the sphere across a relaunch, a broken placeholder took the controller with it, and a broken controller cleared its placeholders"
