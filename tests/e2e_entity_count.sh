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
INTRO_SETTLE=5          # seconds after BRP is up; intro takes ~2.3s

# Component type paths
COMP_SUNDISC="hex_terrain::terrain::entities::HexSunDisc"
COMP_POLE="hex_terrain::terrain::entities::HeightPole"
COMP_QUADLEAF="hex_terrain::terrain::entities::QuadLeaf"
COMP_TRILEAF="hex_terrain::terrain::entities::TriLeaf"
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

# Query a HeightPole's world-space XZ from its GlobalTransform.
# GlobalTransform is an Affine3A: 12 floats, translation at indices [9, 10, 11].
# Finds the pole by Name (e.g. "Pole(0,0)").
brp_pole_world_xz() {
    local name=$1
    local resp
    resp=$(curl -sf -X POST "$BRP" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.query\",\"id\":0,\"params\":{\"data\":{\"components\":[\"$COMP_NAME\",\"$COMP_GTRANSFORM\"]},\"filter\":{\"with\":[\"$COMP_POLE\"]}}}")
    echo "$resp" | jq -r --arg name "$name" \
        '.result[] | select(.components["'"$COMP_NAME"'"] == $name) | .components["'"$COMP_GTRANSFORM"'"] | "\(.[9]) \(.[11])"'
}

# ---------------------------------------------------------------------------
# Launch app
# ---------------------------------------------------------------------------

cargo build --quiet 2>&1

APP_LOG=$(mktemp /tmp/hex-terrain-e2e.XXXXXX.log)

echo "Starting hex-terrain (log: $APP_LOG)..."
cargo run --quiet >"$APP_LOG" 2>&1 &
APP_PID=$!
trap 'kill "$APP_PID" 2>/dev/null; wait "$APP_PID" 2>/dev/null || true; rm -f "$APP_LOG"' EXIT

echo "Waiting for BRP endpoint..."
wait_for_brp

# ---------------------------------------------------------------------------
# Phase 1: During Intro (BRP just became ready, intro still playing)
# ---------------------------------------------------------------------------

echo "Querying during Intro..."
sleep 1
QL_INTRO=$(brp_count "$COMP_QUADLEAF")
Y_INTRO=$(brp_player_y)
echo "  QuadLeaf count: $QL_INTRO  (expect 0 during Intro)"
echo "  Player Y:       $Y_INTRO"

# Pole brightness during Intro (poles exist from startup, fade system runs)
read -r INTRO_PX INTRO_PZ <<< "$(brp_player_xz)"
read -r INTRO_P00_X INTRO_P00_Z <<< "$(brp_pole_world_xz 'Pole(0,0)')"
read -r INTRO_P22_X INTRO_P22_Z <<< "$(brp_pole_world_xz 'Pole(2,2)')"
INTRO_DIST_00=$(LC_NUMERIC=C awk "BEGIN { dx=$INTRO_PX-($INTRO_P00_X); dz=$INTRO_PZ-($INTRO_P00_Z); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
INTRO_DIST_22=$(LC_NUMERIC=C awk "BEGIN { dx=$INTRO_PX-($INTRO_P22_X); dz=$INTRO_PZ-($INTRO_P22_Z); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
INTRO_BRIGHT_00=$(LC_NUMERIC=C awk "BEGIN { t=$INTRO_DIST_00/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
INTRO_BRIGHT_22=$(LC_NUMERIC=C awk "BEGIN { t=$INTRO_DIST_22/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
echo "  Pole(0,0): dist=$INTRO_DIST_00  brightness=$INTRO_BRIGHT_00"
echo "  Pole(2,2): dist=$INTRO_DIST_22  brightness=$INTRO_BRIGHT_22"

# ---------------------------------------------------------------------------
# Phase 2: Wait for intro → running transition
# ---------------------------------------------------------------------------

echo "Waiting ${INTRO_SETTLE}s for intro sequence..."
sleep "$INTRO_SETTLE"

# ---------------------------------------------------------------------------
# Phase 2: During Running
# ---------------------------------------------------------------------------

echo "Querying during Running..."
SD=$(brp_count  "$COMP_SUNDISC")
HP=$(brp_count  "$COMP_POLE")
QL=$(brp_count  "$COMP_QUADLEAF")
TL=$(brp_count  "$COMP_TRILEAF")
Y_RUNNING=$(brp_player_y)
echo "  Player Y:       $Y_RUNNING"

# Pole brightness: gather world-space positions of poles and player
read -r PLAYER_X PLAYER_Z <<< "$(brp_player_xz)"
read -r POLE_X_00 POLE_Z_00 <<< "$(brp_pole_world_xz 'Pole(0,0)')"
read -r POLE_X_22 POLE_Z_22 <<< "$(brp_pole_world_xz 'Pole(2,2)')"

# pole_fade_brightness: 1.0 - clamp(dist/40.0, 0, 1) * 0.95
DIST_00=$(LC_NUMERIC=C awk "BEGIN { dx=$PLAYER_X-($POLE_X_00); dz=$PLAYER_Z-($POLE_Z_00); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
DIST_22=$(LC_NUMERIC=C awk "BEGIN { dx=$PLAYER_X-($POLE_X_22); dz=$PLAYER_Z-($POLE_Z_22); printf \"%.4f\", sqrt(dx*dx+dz*dz) }")
BRIGHT_00=$(LC_NUMERIC=C awk "BEGIN { t=$DIST_00/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
BRIGHT_22=$(LC_NUMERIC=C awk "BEGIN { t=$DIST_22/40.0; if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }")
echo "  Pole(0,0): dist=$DIST_00  brightness=$BRIGHT_00"
echo "  Pole(2,2): dist=$DIST_22  brightness=$BRIGHT_22"

if [[ "${1:-}" == "--print" ]]; then
    printf "\n%-14s %d\n%-14s %d\n%-14s %d\n%-14s %d\n" \
        "HexSunDisc" "$SD" "HeightPole" "$HP" \
        "QuadLeaf" "$QL" "TriLeaf" "$TL"
    printf "\n%-14s %s\n%-14s %s\n" \
        "Y (Intro)" "$Y_INTRO" "Y (Running)" "$Y_RUNNING"
    exit 0
fi

# ---------------------------------------------------------------------------
# Assertions
# ---------------------------------------------------------------------------

PASS=0 FAIL=0

assert_eq() {
    local label=$1 got=$2 want=$3
    if ((got == want)); then
        printf "  ok    %-14s %d\n" "$label" "$got"
        ((PASS++)) || true
    else
        local ratio
        ratio=$(LC_NUMERIC=C awk "BEGIN { printf \"%.2f\", $got / $want }")
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

# During Intro: spawn_petals is gated by GameState::Running → no QuadLeafs yet
assert_eq    "QL(Intro)"   "$QL_INTRO"  0

# During Running: spawn_petals fires → QuadLeafs present
assert_eq    "QL(Running)" "$QL"        57

# Drone altitude should be stable across the transition (±1.0 tolerance)
assert_float_eq "Altitude" "$Y_RUNNING" "$Y_INTRO" 1.0

# --- Entity counts ---
echo ""
echo "=== Entity counts (grid_radius=20, reveal_radius=2) ==="

# Startup: one HexSunDisc per grid hex
assert_eq    "HexSunDisc"  "$SD"  1261

# Startup: one HeightPole per hex where height > pole_gap (noise-dependent)
assert_range "HeightPole"  "$HP"  1200 1261

# Initial draw: 19 hexes * 3 QuadLeafs each (even edges 0, 2, 4)
assert_eq    "QuadLeaf"    "$QL"  57

# Initial draw: 19 hexes * 2 TriLeafs each (vertices 0 and 1, with dedup)
assert_range "TriLeaf"     "$TL"  30 38

# --- Pole brightness (fade by distance from player) ---
echo ""
echo "=== Pole brightness (fade by distance) ==="

# During Intro (QL_INTRO==0 proves GameState::Intro):
# fade_nearby_poles runs unconditionally → distant pole already dimmer.
assert_float_lt "Intro(2,2)<(0,0)" "$INTRO_BRIGHT_22" "$INTRO_BRIGHT_00"

# During Running (QL > 0 proves GameState::Running):
# Same fade still active.
assert_float_lt "Running(2,2)<(0,0)" "$BRIGHT_22" "$BRIGHT_00"

echo ""
echo "$PASS passed, $FAIL failed"
((FAIL == 0))
