# e2e test: launch app, collect BRP data per phase, then assert sequentially.
# Usage: make -f tests/e2e.mk
SHELL := /bin/bash
.PHONY: all phase1 phase2

BRP            := http://127.0.0.1:15702
INTRO_DURATION := 3

TMPDIR       := $(shell mktemp -d /tmp/hex-e2e.XXXXXX)
DATA_INTRO   := $(TMPDIR)/intro.env
DATA_RUNNING := $(TMPDIR)/running.env
PID_FILE     := $(TMPDIR)/app.pid

# Component type paths
C_SD   := hex_terrain::terrain::entities::HexSunDisc
C_STEM := hex_terrain::terrain::entities::Stem
C_QP   := hex_terrain::terrain::entities::QuadPetal
C_TP   := hex_terrain::terrain::entities::TriPetal
C_PLAY := hex_terrain::drone::entities::Player
C_TF   := bevy_transform::components::transform::Transform
C_GTF  := bevy_transform::components::global_transform::GlobalTransform
C_NAME := bevy_ecs::name::Name

# $(comma) hides literal commas from Make's $(call) arg scanner
comma := ,
S00   := Stem(0$(comma)0)
S22   := Stem(2$(comma)2)

# ---------------------------------------------------------------------------
# BRP macros — expand to shell fragments for use in $$(...) subshells
# ---------------------------------------------------------------------------

brp_curl = curl -sf -X POST "$(BRP)" -H "Content-Type: application/json"

# $(call jsonrpc,method,params) → JSON-RPC request body
jsonrpc = {"jsonrpc":"2.0","method":"$(1)","id":0,"params":$(2)}

# $(call qparams,components) → world.query params (no filter)
qparams = {"data":{"components":[$(1)]}}

# $(call qparams_with,components,with) → world.query params with filter
qparams_with = {"data":{"components":[$(1)]},"filter":{"with":[$(2)]}}

# $(call brp_post,method,params) → curl command posting JSON-RPC
brp_post = $(brp_curl) -d '$(call jsonrpc,$(1),$(2))'

# $(call brp_count,component) → pipeline producing integer count
brp_count = $(call brp_post,world.query,$(call qparams,"$(1)")) | jq '.result | length'

# $(call brp_player_y) → pipeline producing player Y coordinate
brp_player_y = $(call brp_post,world.query,$(call qparams_with,"$(C_TF)","$(C_PLAY)")) \
               | jq -r '.result[0].components["$(C_TF)"].translation[1]'

# $(call brp_player_xz) → pipeline producing "X Z"
brp_player_xz = $(call brp_post,world.query,$(call qparams_with,"$(C_TF)","$(C_PLAY)")) \
                | jq -r '.result[0].components["$(C_TF)"].translation | "\(.[0]) \(.[2])"'

# $(call brp_stem_xz,name) → pipeline producing "X Z" for a named Stem
brp_stem_xz = $(call brp_post,world.query,$(call qparams_with,"$(C_NAME)"$(comma)"$(C_GTF)","$(C_STEM)")) \
              | jq -r --arg n '$(1)' '.result[] | select(.components["$(C_NAME)"] == $$n) | .components["$(C_GTF)"] | "\(.[9]) \(.[11])"'

# $(call stem_brightness,$$px,$$pz,$$sx,$$sz) → awk computing brightness from distance
stem_brightness = LC_NUMERIC=C awk "BEGIN { t=sqrt(($(1)-($(3)))^2+($(2)-($(4)))^2)/40.0; \
                  if(t>1)t=1; if(t<0)t=0; printf \"%.4f\", 1.0-t*0.95 }"

# $(call collect_brightness,VAR_PREFIX) → shell block setting PREFIX_00 and PREFIX_22
collect_brightness = \
    read -r _PX _PZ <<< "$$($(call brp_player_xz))"; \
    read -r _S0X _S0Z <<< "$$($(call brp_stem_xz,$(S00)))"; \
    read -r _S2X _S2Z <<< "$$($(call brp_stem_xz,$(S22)))"; \
    $(1)_00=$$($(call stem_brightness,$$_PX,$$_PZ,$$_S0X,$$_S0Z)); \
    $(1)_22=$$($(call stem_brightness,$$_PX,$$_PZ,$$_S2X,$$_S2Z))

# ---------------------------------------------------------------------------
# Assert macros — expand to inline shell statements; share PASS/FAIL/SKIP vars
# ---------------------------------------------------------------------------

# $(call assert_skip,label,reason)
assert_skip = printf "  skip  %-14s (%s)\n" "$(1)" "$(2)"; ((SKIP++)) || true

# $(call assert_eq,label,got,want)
assert_eq = if (($(2) == $(3))); then \
    printf "  ok    %-14s %d\n" "$(1)" "$(2)"; ((PASS++)) || true; \
    else _r="n/a"; if (($(3) != 0)); then \
    _r=$$(LC_NUMERIC=C awk "BEGIN { printf \"%.2f\", $(2) / $(3) }"); fi; \
    printf "  FAIL  %-14s got %d, expected %d  (ratio: %s)\n" "$(1)" "$(2)" "$(3)" "$$_r"; \
    ((FAIL++)) || true; fi

# $(call assert_range,label,got,lo,hi)
assert_range = if (($(2) >= $(3) && $(2) <= $(4))); then \
    printf "  ok    %-14s %d  (range %d..%d)\n" "$(1)" "$(2)" "$(3)" "$(4)"; ((PASS++)) || true; \
    else printf "  FAIL  %-14s got %d, expected %d..%d\n" "$(1)" "$(2)" "$(3)" "$(4)"; \
    ((FAIL++)) || true; fi

# $(call assert_float_eq,label,got,want,tol)
assert_float_eq = if LC_NUMERIC=C awk "BEGIN { d=$(2)-$(3); if(d<0)d=-d; exit !(d<=$(4)) }"; then \
    LC_NUMERIC=C printf "  ok    %-20s %.2f  (expected ~%.2f ±%s)\n" "$(1)" "$(2)" "$(3)" "$(4)"; \
    ((PASS++)) || true; else \
    LC_NUMERIC=C printf "  FAIL  %-20s got %.2f, expected ~%.2f ±%s\n" "$(1)" "$(2)" "$(3)" "$(4)"; \
    ((FAIL++)) || true; fi

# $(call assert_float_lt,label,a,b)
assert_float_lt = if LC_NUMERIC=C awk "BEGIN { exit !($(2) < $(3)) }"; then \
    LC_NUMERIC=C printf "  ok    %-20s %.4f < %.4f\n" "$(1)" "$(2)" "$(3)"; \
    ((PASS++)) || true; else \
    LC_NUMERIC=C printf "  FAIL  %-20s %.4f not < %.4f\n" "$(1)" "$(2)" "$(3)"; \
    ((FAIL++)) || true; fi

# $(call assert_summary)
assert_summary = echo ""; echo "$$PASS passed, $$FAIL failed, $$SKIP skipped"; ((FAIL == 0))

# ---------------------------------------------------------------------------
# Dependency chain: $(DATA_INTRO) → $(DATA_RUNNING) → phase1 → phase2 → all
# ---------------------------------------------------------------------------

all: phase1 phase2
	@rm -rf $(TMPDIR)

phase2: phase1

# ---------------------------------------------------------------------------
# $(DATA_INTRO): build, launch app, wait for BRP, collect intro snapshot
# ---------------------------------------------------------------------------

$(DATA_INTRO):
	@set -euo pipefail; \
	cargo build --quiet 2>&1; \
	if pgrep -x hex-terrain >/dev/null 2>&1; then \
	    echo "Killing stale hex-terrain process(es)..."; \
	    pkill -x hex-terrain; sleep 1; \
	fi; \
	echo "Starting hex-terrain (log: $(TMPDIR)/app.log)..."; \
	cargo run --quiet -- --intro-duration $(INTRO_DURATION) >$(TMPDIR)/app.log 2>&1 & \
	echo $$! > $(PID_FILE); \
	echo "Waiting for BRP endpoint..."; \
	tries=0; \
	while ! curl -sf -o /dev/null -X POST "$(BRP)" \
	    -H "Content-Type: application/json" \
	    -d '{"jsonrpc":"2.0","method":"rpc.discover","id":0}' 2>/dev/null; do \
	    ((tries++)) || true; \
	    if ((tries > 30)); then \
	        echo "ERROR: BRP not responding after 30 attempts" >&2; \
	        tail -20 $(TMPDIR)/app.log >&2; exit 1; \
	    fi; \
	    sleep 1; \
	done; \
	echo "--- Collecting intro data ---"; \
	CAUGHT_INTRO=false; QUAD_PETAL_INTRO=0; Y_INTRO=""; \
	INTRO_BRIGHT_00=""; INTRO_BRIGHT_22=""; \
	for _ in $$(seq 1 50); do \
	    ql=$$($(call brp_count,$(C_QP))); \
	    if ((ql == 0)); then \
	        CAUGHT_INTRO=true; QUAD_PETAL_INTRO=0; \
	        Y_INTRO=$$($(call brp_player_y)); \
	        $(call collect_brightness,INTRO_BRIGHT); \
	        echo "  Caught Intro: QL=0  Player Y=$$Y_INTRO"; \
	        break; \
	    fi; \
	    sleep 0.1; \
	done; \
	if ! $$CAUGHT_INTRO; then \
	    echo "  WARNING: missed Intro window"; \
	    QUAD_PETAL_INTRO=$$($(call brp_count,$(C_QP))); \
	    Y_INTRO=$$($(call brp_player_y)); \
	fi; \
	{ echo "CAUGHT_INTRO=$$CAUGHT_INTRO"; \
	  echo "QUAD_PETAL_INTRO=$$QUAD_PETAL_INTRO"; \
	  echo "Y_INTRO=$$Y_INTRO"; \
	  echo "INTRO_BRIGHT_00=$$INTRO_BRIGHT_00"; \
	  echo "INTRO_BRIGHT_22=$$INTRO_BRIGHT_22"; \
	} > $@

# ---------------------------------------------------------------------------
# $(DATA_RUNNING): poll for Running state, collect snapshot, kill app
# ---------------------------------------------------------------------------

$(DATA_RUNNING): $(DATA_INTRO)
	@set -euo pipefail; \
	trap 'kill $$(cat $(PID_FILE)) 2>/dev/null; wait $$(cat $(PID_FILE)) 2>/dev/null || true' EXIT; \
	echo "--- Collecting running data ---"; \
	for _ in $$(seq 1 100); do \
	    ql=$$($(call brp_count,$(C_QP))); \
	    if ((ql > 0)); then echo "  Caught Running: QL=$$ql"; break; fi; \
	    sleep 0.1; \
	done; \
	SUN_DISC=$$($(call brp_count,$(C_SD))); \
	STEM=$$($(call brp_count,$(C_STEM))); \
	QUAD_PETAL=$$($(call brp_count,$(C_QP))); \
	TRI_PETAL=$$($(call brp_count,$(C_TP))); \
	Y_RUNNING=$$($(call brp_player_y)); \
	echo "  Player Y: $$Y_RUNNING"; \
	$(call collect_brightness,BRIGHT); \
	{ echo "SUN_DISC=$$SUN_DISC"; \
	  echo "STEM=$$STEM"; \
	  echo "QUAD_PETAL=$$QUAD_PETAL"; \
	  echo "TRI_PETAL=$$TRI_PETAL"; \
	  echo "Y_RUNNING=$$Y_RUNNING"; \
	  echo "BRIGHT_00=$$BRIGHT_00"; \
	  echo "BRIGHT_22=$$BRIGHT_22"; \
	} > $@

# ---------------------------------------------------------------------------
# Phase 1: Intro assertions (skipped if Intro window was missed)
# ---------------------------------------------------------------------------

phase1: $(DATA_INTRO)
	@set -euo pipefail; \
	PASS=0; FAIL=0; SKIP=0; \
	source $<; \
	echo ""; \
	echo "=== Phase 1: Intro ==="; \
	if [[ "$$CAUGHT_INTRO" == "true" ]]; then \
	    $(call assert_eq,QuadPetal,$$QUAD_PETAL_INTRO,0); \
	    $(call assert_eq,Revealed(Intro),$$((QUAD_PETAL_INTRO / 3)),0); \
	    $(call assert_float_eq,Altitude,$$Y_INTRO,12.0,1.0); \
	    $(call assert_float_lt,Intro(2$(comma)2)<(0$(comma)0),$$INTRO_BRIGHT_22,$$INTRO_BRIGHT_00); \
	else \
	    $(call assert_skip,QuadPetal,missed Intro window); \
	    $(call assert_skip,Revealed(Intro),missed Intro window); \
	    $(call assert_skip,Altitude,missed Intro window); \
	    $(call assert_skip,Intro(2$(comma)2)<(0$(comma)0),missed Intro window); \
	fi; \
	$(call assert_summary)

# ---------------------------------------------------------------------------
# Phase 2: Running assertions (entity counts, reveal, stem brightness)
# ---------------------------------------------------------------------------

phase2: $(DATA_RUNNING)
	@set -euo pipefail; \
	PASS=0; FAIL=0; SKIP=0; \
	source $<; \
	echo ""; \
	echo "=== Phase 2: Running ==="; \
	$(call assert_eq,HexSunDisc,$$SUN_DISC,1261); \
	$(call assert_range,Stem,$$STEM,1200,1261); \
	$(call assert_eq,QuadPetal,$$QUAD_PETAL,57); \
	$(call assert_range,TriPetal,$$TRI_PETAL,30,38); \
	$(call assert_eq,Revealed(Running),$$((QUAD_PETAL / 3)),19); \
	$(call assert_float_eq,Altitude,$$Y_RUNNING,12.0,1.0); \
	$(call assert_float_lt,Running(2$(comma)2)<(0$(comma)0),$$BRIGHT_22,$$BRIGHT_00); \
	$(call assert_summary)
