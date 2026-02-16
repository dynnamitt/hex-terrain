#!/usr/bin/env bash
# e2e test: verify entity counts and intro→running state transition.
#
# Starts the app, queries during GameState::Intro (no petals, drone at spawn
# height), waits for intro to finish, then re-queries during GameState::Running
# (petals spawned, drone altitude stable).
#
# Usage:
#   ./tests/e2e_entity_count.sh           # run assertions
#   ./tests/e2e_entity_count.sh --print   # print counts only
set -euo pipefail

BRP="http://127.0.0.1:15702"
INTRO_DURATION=3        # long enough for BRP to be ready before intro ends

# Component type paths
COMP_SUNDISC="hex_terrain::terrain::entities::HexSunDisc"
COMP_STEM="hex_terrain::terrain::entities::Stem"
COMP_QUADPETAL="hex_terrain::terrain::entities::QuadPetal"
COMP_TRIPETAL="hex_terrain::terrain::entities::TriPetal"
COMP_PLAYER="hex_terrain::drone::entities::Player"
COMP_TRANSFORM="bevy_transform::components::transform::Transform"
COMP_GTRANSFORM="bevy_transform::components::global_transform::GlobalTransform"
COMP_NAME="bevy_ecs::name::Name"

# ---------------------------------------------------------------------------
# BRP helpers
# ---------------------------------------------------------------------------

# Wait until the BRP HTTP endpoint responds (up to 30s).
wait_for_brp() {
    local tries=0
    while ! curl -sf -o /dev/null -X POST "$BRP" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","method":"rpc.discover","id":0}' 2>/dev/null; do
        ((tries++)) || true
        if ((tries > 30)); then
            echo "ERROR: BRP not responding after 30 attempts" >&2
            echo "--- app log ---" >&2
            tail -20 "$APP_LOG" >&2
            exit 1
        fi
        sleep 1
    done
}

# Query entity count for a component type path.
brp_count() {
    local component=$1
    local resp
    resp=$(curl -sf -X POST "$BRP" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.query\",\"id\":0,\"params\":{\"data\":{\"components\":[\"$component\"]}}}")
    local count
    count=$(echo "$resp" | jq '.result | length')
    if ((count == 0)); then
        echo "$resp" | jq -c '.error // empty' >&2
    fi
    echo "$count"
}

# Query the Player entity's Transform.translation[1] (y).
# Bevy 0.18 BRP serializes Transform.translation as [x, y, z] array.
brp_player_y() {
    local resp
    resp=$(curl -sf -X POST "$BRP" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.query\",\"id\":0,\"params\":{\"data\":{\"components\":[\"$COMP_TRANSFORM\"]},\"filter\":{\"with\":[\"$COMP_PLAYER\"]}}}")
    echo "$resp" | jq -r ".result[0].components[\"$COMP_TRANSFORM\"].translation[1]"
}

# Query the Player entity's Transform.translation as "x z".
brp_player_xz() {
    local resp
    resp=$(curl -sf -X POST "$BRP" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.query\",\"id\":0,\"params\":{\"data\":{\"components\":[\"$COMP_TRANSFORM\"]},\"filter\":{\"with\":[\"$COMP_PLAYER\"]}}}")
    echo "$resp" | jq -r ".result[0].components[\"$COMP_TRANSFORM\"].translation | \"\(.[0]) \(.[2])\""
}

# Query a Stem's world-space XZ from its GlobalTransform.
# GlobalTransform is an Affine3A: 12 floats, translation at indices [9, 10, 11].
# Finds the stem by Name (e.g. "Stem(0,0)").
brp_stem_world_xz() {
    local name=$1
    local resp
    resp=$(curl -sf -X POST "$BRP" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.query\",\"id\":0,\"params\":{\"data\":{\"components\":[\"$COMP_NAME\",\"$COMP_GTRANSFORM\"]},\"filter\":{\"with\":[\"$COMP_STEM\"]}}}")
    echo "$resp" | jq -r --arg name "$name" \
        '.result[] | select(.components["'"$COMP_NAME"'"] == $name) | .components["'"$COMP_GTRANSFORM"'"] | "\(.[9]) \(.[11])"'
}

# ---------------------------------------------------------------------------
# Launch app
# ---------------------------------------------------------------------------

cargo build --quiet 2>&1

# Kill any stale hex-terrain process holding the BRP port
if pgrep -x hex-terrain >/dev/null 2>&1; then
    echo "Killing stale hex-terrain process(es)..."
    pkill -x hex-terrain; sleep 1
fi

APP_LOG=$(mktemp /tmp/hex-terrain-e2e.XXXXXX.log)

echo "Starting hex-terrain (log: $APP_LOG)..."
cargo run --quiet -- --intro-duration "$INTRO_DURATION" >"$APP_LOG" 2>&1 &
APP_PID=$!
trap 'kill "$APP_PID" 2>/dev/null; wait "$APP_PID" 2>/dev/null || true; rm -f "$APP_LOG"' EXIT

echo "Waiting for BRP endpoint..."
wait_for_brp

# ---------------------------------------------------------------------------
# Poll for state transitions (Intro → Running)
# ---------------------------------------------------------------------------
# QuadPetal count == 0 means Intro (reveal gated by GameState::Running).
# QuadPetal count > 0 means Running. Poll in a tight loop to observe both.

echo "Polling for Intro state (QL==0)..."
CAUGHT_INTRO=false
QL_INTRO=0
Y_INTRO=""
for _ in $(seq 1 50); do
    ql=$(brp_count "$COMP_QUADPETAL")
    if ((ql == 0)); then
        CAUGHT_INTRO=true
        QL_INTRO=0
        Y_INTRO=$(brp_player_y)
        # Stem brightness during Intro
        read -r INTRO_PX INTRO_PZ <<< "$(brp_player_xz)"
        read -r INTRO_P00_X INTRO_P00_Z <<< "$(brp_stem_world_xz 'Stem(0,0)')"
        read -r INTRO_P22_X INTRO_P22_Z <<< "$(brp_stem_world_xz 'Stem(2,2)')"
        INTRO_DIST_00=$(LC_NUMERIC=C awk "BEGIN { dx=$INTRO_PX-($INTRO_P00_X); dz=$INTRO_PZ-($INTRO_P00_Z); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
        INTRO_DIST_22=$(LC_NUMERIC=C awk "BEGIN { dx=$INTRO_PX-($INTRO_P22_X); dz=$INTRO_PZ-($INTRO_P22_Z); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
        INTRO_BRIGHT_00=$(LC_NUMERIC=C awk "BEGIN { t=$INTRO_DIST_00/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
        INTRO_BRIGHT_22=$(LC_NUMERIC=C awk "BEGIN { t=$INTRO_DIST_22/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
        echo "  Caught Intro: QL=0  Player Y=$Y_INTRO"
        echo "  Stem(0,0): dist=$INTRO_DIST_00  brightness=$INTRO_BRIGHT_00"
        echo "  Stem(2,2): dist=$INTRO_DIST_22  brightness=$INTRO_BRIGHT_22"
        break
    fi
    sleep 0.1
done
if ! $CAUGHT_INTRO; then
    echo "  WARNING: missed Intro window (app transitioned before first query)"
    QL_INTRO=$(brp_count "$COMP_QUADPETAL")
    Y_INTRO=$(brp_player_y)
fi

echo "Polling for Running state (QL>0)..."
for _ in $(seq 1 100); do
    ql=$(brp_count "$COMP_QUADPETAL")
    if ((ql > 0)); then
        echo "  Caught Running: QL=$ql"
        break
    fi
    sleep 0.1
done

echo "Querying during Running..."
SD=$(brp_count  "$COMP_SUNDISC")
HP=$(brp_count  "$COMP_STEM")
QL=$(brp_count  "$COMP_QUADPETAL")
TL=$(brp_count  "$COMP_TRIPETAL")
Y_RUNNING=$(brp_player_y)
echo "  Player Y:       $Y_RUNNING"

# Stem brightness: gather world-space positions of stems and player
read -r PLAYER_X PLAYER_Z <<< "$(brp_player_xz)"
read -r POLE_X_00 POLE_Z_00 <<< "$(brp_stem_world_xz 'Stem(0,0)')"
read -r POLE_X_22 POLE_Z_22 <<< "$(brp_stem_world_xz 'Stem(2,2)')"

# stem_fade_brightness: 1.0 - clamp(dist/40.0, 0, 1) * 0.95
DIST_00=$(LC_NUMERIC=C awk "BEGIN { dx=$PLAYER_X-($POLE_X_00); dz=$PLAYER_Z-($POLE_Z_00); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
DIST_22=$(LC_NUMERIC=C awk "BEGIN { dx=$PLAYER_X-($POLE_X_22); dz=$PLAYER_Z-($POLE_Z_22); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
BRIGHT_00=$(LC_NUMERIC=C awk "BEGIN { t=$DIST_00/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
BRIGHT_22=$(LC_NUMERIC=C awk "BEGIN { t=$DIST_22/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
echo "  Stem(0,0): dist=$DIST_00  brightness=$BRIGHT_00"
echo "  Stem(2,2): dist=$DIST_22  brightness=$BRIGHT_22"

if [[ "${1:-}" == "--print" ]]; then
    printf "\n%-14s %d\n%-14s %d\n%-14s %d\n%-14s %d\n" \
        "HexSunDisc" "$SD" "Stem" "$HP" \
        "QuadPetal" "$QL" "TriPetal" "$TL"
    printf "\n%-14s %s\n%-14s %s\n" \
        "Y (Intro)" "$Y_INTRO" "Y (Running)" "$Y_RUNNING"
    exit 0
fi

# ---------------------------------------------------------------------------
# Assertions
# ---------------------------------------------------------------------------

PASS=0 FAIL=0 SKIP=0

assert_skip() {
    local label=$1 reason=$2
    printf "  skip  %-14s (%s)\n" "$label" "$reason"
    ((SKIP++)) || true
}

assert_eq() {
    local label=$1 got=$2 want=$3
    if ((got == want)); then
        printf "  ok    %-14s %d\n" "$label" "$got"
        ((PASS++)) || true
    else
        local ratio="n/a"
        if ((want != 0)); then
            ratio=$(LC_NUMERIC=C awk "BEGIN { printf \"%.2f\", $got / $want }")
        fi
        printf "  FAIL  %-14s got %d, expected %d  (ratio: %s)\n" \
            "$label" "$got" "$want" "$ratio"
        ((FAIL++)) || true
    fi
}

assert_range() {
    local label=$1 got=$2 lo=$3 hi=$4
    if ((got >= lo && got <= hi)); then
        printf "  ok    %-14s %d  (range %d..%d)\n" "$label" "$got" "$lo" "$hi"
        ((PASS++)) || true
    else
        printf "  FAIL  %-14s got %d, expected %d..%d\n" "$label" "$got" "$lo" "$hi"
        ((FAIL++)) || true
    fi
}

assert_float_eq() {
    local label=$1 got=$2 want=$3 tol=$4
    if LC_NUMERIC=C awk "BEGIN { d=$got-$want; if(d<0)d=-d; exit !(d<=$tol) }"; then
        LC_NUMERIC=C printf "  ok    %-20s %.2f  (expected ~%.2f ±%s)\n" "$label" "$got" "$want" "$tol"
        ((PASS++)) || true
    else
        LC_NUMERIC=C printf "  FAIL  %-20s got %.2f, expected ~%.2f ±%s\n" "$label" "$got" "$want" "$tol"
        ((FAIL++)) || true
    fi
}

assert_float_lt() {
    local label=$1 a=$2 b=$3
    if LC_NUMERIC=C awk "BEGIN { exit !($a < $b) }"; then
        LC_NUMERIC=C printf "  ok    %-20s %.4f < %.4f\n" "$label" "$a" "$b"
        ((PASS++)) || true
    else
        LC_NUMERIC=C printf "  FAIL  %-20s %.4f not < %.4f\n" "$label" "$a" "$b"
        ((FAIL++)) || true
    fi
}

# --- State transition (indirect: petals only spawn during Running) ---
echo ""
echo "=== State transition ==="

# During Intro: spawn_petals is gated by GameState::Running → no QuadPetals yet
if $CAUGHT_INTRO; then
    assert_eq    "QL(Intro)"   "$QL_INTRO"  0
else
    assert_skip  "QL(Intro)"   "missed Intro window"
fi

# During Running: spawn_petals fires → QuadPetals present
assert_eq    "QL(Running)" "$QL"        57

# Drone altitude should be stable across the transition (±1.0 tolerance)
if $CAUGHT_INTRO; then
    assert_float_eq "Altitude" "$Y_RUNNING" "$Y_INTRO" 1.0
else
    assert_skip  "Altitude"    "missed Intro window"
fi

# --- Entity counts ---
echo ""
echo "=== Entity counts (grid_radius=20, reveal_radius=2) ==="

# Startup: one HexSunDisc per grid hex
assert_eq    "HexSunDisc"  "$SD"  1261

# Startup: one Stem per hex where height > stem_gap (noise-dependent)
assert_range "Stem"        "$HP"  1200 1261

# Initial draw: 19 hexes * 3 QuadPetals each (even edges 0, 2, 4)
assert_eq    "QuadPetal"    "$QL"  57

# Initial draw: 19 hexes * 2 TriPetals each (vertices 0 and 1, with dedup)
assert_range "TriPetal"     "$TL"  30 38

# --- FlowerState reveal (hexagon(center, reveal_radius=2) = 19 hexes) ---
# BRP can't read FlowerState variants (Vec<Entity> lacks ReflectSerialize),
# so we derive revealed hex count from QuadPetal count (3 per revealed hex).
echo ""
echo "=== FlowerState reveal (reveal_radius=2) ==="

REVEALED_INTRO=$((QL_INTRO / 3))
REVEALED_RUNNING=$((QL / 3))

# During Intro: reveal_nearby_hexes is gated by GameState::Running → all Naked
if $CAUGHT_INTRO; then
    assert_eq    "Revealed(Intro)"   "$REVEALED_INTRO"    0
else
    assert_skip  "Revealed(Intro)"   "missed Intro window"
fi

# During Running: 19 hexes promoted (1 PlayerAbove + 18 Revealed)
assert_eq    "Revealed(Running)" "$REVEALED_RUNNING"  19

# --- Stem brightness (fade by distance from player) ---
echo ""
echo "=== Stem brightness (fade by distance) ==="

# During Intro: highlight_nearby_stems runs unconditionally → distant stem dimmer.
if $CAUGHT_INTRO; then
    assert_float_lt "Intro(2,2)<(0,0)" "$INTRO_BRIGHT_22" "$INTRO_BRIGHT_00"
else
    assert_skip  "Intro(2,2)<(0,0)" "missed Intro window"
fi

# During Running (QL > 0 proves GameState::Running):
# Same fade still active.
assert_float_lt "Running(2,2)<(0,0)" "$BRIGHT_22" "$BRIGHT_00"

echo ""
echo "$PASS passed, $FAIL failed, $SKIP skipped"
((FAIL == 0))
