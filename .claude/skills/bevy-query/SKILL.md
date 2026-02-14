---
name: bevy-query
description: Query running Bevy app ECS state via the remote protocol
---

Connect to the Bevy remote inspector at http://127.0.0.1:15702 via JSON-RPC.

## Methods (Bevy 0.18)

| Method | Purpose |
|---|---|
| `rpc.discover` | List available methods |
| `world.query` | Query entities by components, with optional filter |
| `world.get` | Get components from a specific entity by ID |
| `world.list` | List all registered component types |
| `world.spawn` | Spawn a new entity |
| `world.despawn` | Despawn an entity |
| `world.insert` | Insert components on an entity |
| `world.remove` | Remove components from an entity |
| `world.reparent` | Change entity parent |

## Type path conventions

Component types use their full Rust module path (where defined, not re-exported):

- Crate types: `hex_terrain::terrain::entities::HexSunDisc`
- Bevy Transform: `bevy_transform::components::transform::Transform`
- Marker components: `hex_terrain::drone::entities::Player`

## Query examples

### Count entities with a component

```json
{
  "jsonrpc": "2.0", "method": "world.query", "id": 0,
  "params": {
    "data": {
      "components": ["hex_terrain::terrain::entities::HexSunDisc"]
    }
  }
}
```

Response: `.result` is an array — use `jq '.result | length'` to count.

### Query component data with a filter

Fetch Transform on entities that have a Player marker:

```json
{
  "jsonrpc": "2.0", "method": "world.query", "id": 0,
  "params": {
    "data": {
      "components": ["bevy_transform::components::transform::Transform"]
    },
    "filter": {
      "with": ["hex_terrain::drone::entities::Player"]
    }
  }
}
```

Response structure:
```json
{
  "result": [
    {
      "entity": "4294967298v1",
      "components": {
        "bevy_transform::components::transform::Transform": {
          "translation": { "x": 0.0, "y": 12.0, "z": 0.0 },
          "rotation": [0.0, 0.0, 0.0, 1.0],
          "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
        }
      }
    }
  ]
}
```

Extract a field: `jq '.result[0].components["...Transform"].translation.y'`

### Query params reference

```json
{
  "data": {
    "components": ["..."],  // components to return data for (required)
    "option": ["..."],      // optional components (may be absent)
    "has": ["..."]          // boolean presence check (no data returned)
  },
  "filter": {
    "with": ["..."],        // required components (not returned in data)
    "without": ["..."]      // excluded components
  }
}
```

## curl one-liner

```bash
curl -sf -X POST http://127.0.0.1:15702 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"world.query","id":0,"params":{"data":{"components":["COMPONENT_PATH"]}}}' \
  | jq '.result | length'
```
