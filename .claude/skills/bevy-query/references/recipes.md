# BRP Recipes

Practical curl + jq patterns for querying hex-terrain via BRP.

## Entity counting

```bash
# Count all HexSunDisc entities
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.query","id":0,
       "params":{"data":{"components":["hex_terrain::terrain::entities::HexSunDisc"]}}}' \
  | jq '.result | length'
```

## Read a component field

```bash
# Get Player's world position (Transform.translation)
COMP="bevy_transform::components::transform::Transform"
FILTER="hex_terrain::drone::entities::Player"
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.query\",\"id\":0,
       \"params\":{\"data\":{\"components\":[\"$COMP\"]},
                   \"filter\":{\"with\":[\"$FILTER\"]}}}" \
  | jq ".result[0].components[\"$COMP\"].translation"
```

Output: `{ "x": 0.0, "y": 12.0, "z": 0.0 }`

## Extract a single numeric field

```bash
# Player Y altitude
... | jq -r ".result[0].components[\"$COMP\"].translation.y"
```

## Query multiple components at once

```bash
# Get both Transform and Name for Player
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.query","id":0,
       "params":{"data":{"components":[
         "bevy_transform::components::transform::Transform",
         "bevy_core::name::Name"
       ]},
       "filter":{"with":["hex_terrain::drone::entities::Player"]}}}' \
  | jq '.result[0].components'
```

## Filter with `without`

```bash
# All named entities that are NOT HexSunDiscs
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.query","id":0,
       "params":{"data":{"components":["bevy_core::name::Name"]},
                 "filter":{"without":["hex_terrain::terrain::entities::HexSunDisc"]}}}' \
  | jq '.result | length'
```

## Get a specific entity by ID

```bash
# First find the entity ID from a query result
ENTITY=$(curl -sf ... | jq -r '.result[0].entity')

# Then fetch specific components
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"world.get\",\"id\":0,
       \"params\":{\"entity\":\"$ENTITY\",
                   \"components\":[\"bevy_transform::components::transform::Transform\"]}}" \
  | jq '.result'
```

## Discover available methods

```bash
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"rpc.discover","id":0}' \
  | jq '.result.methods[].name'
```

## E2e test pattern

The project uses BRP queries in `tests/e2e_entity_count.sh` to verify:

1. **During Intro**: QuadLeaf count = 0 (petals not spawned), capture Player Y
2. **During Running**: QuadLeaf count = 57, verify Player Y stable (±1.0)
3. **Entity counts**: HexSunDisc=1261, HeightPole≈1200-1261, TriLeaf≈30-38

This pattern (query before/after state transition) works for any GameState
verification since different systems are gated by different states.
