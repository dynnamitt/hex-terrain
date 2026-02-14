# BRP Response Format

All responses follow JSON-RPC 2.0.

## Success

```json
{
  "jsonrpc": "2.0",
  "id": 0,
  "result": <method-specific>
}
```

## Error

```json
{
  "jsonrpc": "2.0",
  "id": 0,
  "error": {
    "code": -32600,
    "message": "description of what went wrong"
  }
}
```

## `world.query` result

Array of entity rows. Each row contains:

```json
{
  "entity": "4294967298v1",
  "components": {
    "full::type::Path": { <serialized component data> }
  },
  "has": {
    "full::type::Path": true
  }
}
```

- `entity` — opaque entity identifier (format: `{id}v{generation}`)
- `components` — data for each type listed in `params.data.components`
- `has` — boolean presence for each type listed in `params.data.has`

### Transform component shape

```json
{
  "translation": { "x": 0.0, "y": 12.0, "z": 0.0 },
  "rotation": [0.0, 0.0, 0.0, 1.0],
  "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
}
```

Note: `rotation` is a quaternion `[x, y, z, w]`, not Euler angles.

### Name component shape

```json
"Player"
```

Just a string, not an object.

### Marker component shape (no fields)

```json
{}
```

Empty object for unit structs like `Player`, `HeightPole`, `PetalEdge`.

### Reflect enum shape (e.g. HexSunDisc)

```json
{
  "hex": { "x": 0, "y": 0 }
}
```

Struct fields serialized as JSON objects.

## `world.list` result

Array of registered component type path strings:

```json
["bevy_core::name::Name", "bevy_transform::components::transform::Transform", ...]
```

## `world.get` result

Object with component type paths as keys:

```json
{
  "bevy_transform::components::transform::Transform": {
    "translation": { "x": 0.0, "y": 12.0, "z": 0.0 },
    ...
  }
}
```

## Common error codes

| Code | Meaning |
|---|---|
| -32600 | Invalid request (malformed JSON-RPC) |
| -32601 | Method not found |
| -32602 | Invalid params (wrong type path, missing field) |
| -32603 | Internal error (entity not found, etc.) |

## Debugging tips

- If `world.query` returns `[]` (empty result), the component type path is
  likely wrong. Use `world.list` to find the correct path.
- If you get an error about an unregistered type, ensure the type has
  `#[derive(Reflect)]` and is registered via `app.register_type::<T>()`.
- Entity IDs are not stable across runs — always query by component, not ID.
