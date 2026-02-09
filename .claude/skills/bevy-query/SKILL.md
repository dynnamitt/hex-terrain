---
name: bevy-query
description: Query running Bevy app ECS state via the remote protocol
---

Connect to the Bevy remote inspector at http://127.0.0.1:15702 via JSON-RPC.
Available methods: bevy/list_components, bevy/query, bevy/get.
Accept a natural language query, translate it to the appropriate JSON-RPC call,
and present the results in a readable format.
