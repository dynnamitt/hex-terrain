#!/usr/bin/env bash
# e2e test: verify initial entity spawn counts via Bevy Remote Protocol.
#
# Starts the app, waits for the intro sequence to finish (which triggers
# the first geometry draw at Hex::ZERO with reveal_radius=2), then queries
# component counts over BRP and compares against expected values.
#
# Derived expected values (grid_radius=20, reveal_radius=2, RenderMode::Full):
#
#   HexFace:    3*20*21+1 = 1261  (one per grid hex, spawned at startup)
#   HeightPole: <= 1261           (noise-dependent, skip exact assertion)
#
#   Initial draw covers hexagon(ZERO,2) = 19 hexes, all interior to the
#   radius-20 grid so every hex has all 6 neighbors.
#
#   Per interior hex with correct dedup:
#     Perimeter edges:  6 EdgeLine  (hex outline, no sharing)
#     Owned gap quads:  3 GapFace   (6 edges / 2-way sharing = 3)
#     Owned gap tris:   2 GapFace   (6 vertices / 3-way sharing = 2)
#     Cross-gap lines:  6 EdgeLine  (2 per owned edge; same lines border
#                                    the triangles, no separate tri edges)
#
#   EdgeLine = 19 * (6 + 6)        = 228
#   GapFace  = 19 * (3 + 2)        =  95
#
# Usage:
#   ./tests/e2e_entity_count.sh           # run assertions
#   ./tests/e2e_entity_count.sh --print   # print counts only
set -euo pipefail

BRP="http://127.0.0.1:15702"
INTRO_SETTLE=5          # seconds after BRP is up; intro takes ~2.3s

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

# ---------------------------------------------------------------------------
# Launch app
# ---------------------------------------------------------------------------

cargo build --quiet 2>&1

APP_LOG=$(mktemp /tmp/hex-terrain-e2e.XXXXXX.log)

echo "Starting hex-terrain (log: $APP_LOG)..."
cargo run --quiet -- --mode full >"$APP_LOG" 2>&1 &
APP_PID=$!
trap 'kill "$APP_PID" 2>/dev/null; wait "$APP_PID" 2>/dev/null || true; rm -f "$APP_LOG"' EXIT

echo "Waiting for BRP endpoint..."
wait_for_brp
echo "BRP ready. Waiting ${INTRO_SETTLE}s for intro sequence..."
sleep "$INTRO_SETTLE"

# ---------------------------------------------------------------------------
# Query counts
# ---------------------------------------------------------------------------

echo "Querying entity counts..."
HF=$(brp_count  "hex_terrain::grid::HexFace")
HP=$(brp_count  "hex_terrain::grid::HeightPole")
EL=$(brp_count  "hex_terrain::edges::EdgeLine")
GF=$(brp_count  "hex_terrain::edges::GapFace")

if [[ "${1:-}" == "--print" ]]; then
    printf "\n%-14s %d\n%-14s %d\n%-14s %d\n%-14s %d\n" \
        "HexFace" "$HF" "HeightPole" "$HP" "EdgeLine" "$EL" "GapFace" "$GF"
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

echo ""
echo "=== Entity counts (grid_radius=20, reveal_radius=2, mode=full) ==="

# Startup: one HexFace per grid hex
assert_eq    "HexFace"    "$HF"  1261

# Startup: one HeightPole per hex where height > pole_gap (noise-dependent)
assert_range "HeightPole" "$HP"  1200 1261

# Initial draw: 19 hexes * 12 EdgeLine each (6 perimeter + 6 cross-gap)
assert_eq    "EdgeLine"   "$EL"  228

# Initial draw: 19 hexes * 5 GapFace each (3 quads + 2 triangles)
assert_eq    "GapFace"    "$GF"  95

echo ""
echo "$PASS passed, $FAIL failed"
((FAIL == 0))
