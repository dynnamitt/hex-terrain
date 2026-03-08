[![codecov](https://codecov.io/gh/dynnamitt/hex-terrain/graph/badge.svg)](https://codecov.io/gh/dynnamitt/hex-terrain)

# 3d geometric play with rust,bevy & mrOpus

[![Play in browser](https://img.shields.io/badge/Play_in_browser-654FF0?style=for-the-badge&logo=webassembly&logoColor=white)](https://dynnamitt.github.io/hex-terrain/)

 
          v2----v1
         /       \
       v3          v0
         \        /
          v4----v5

## 1st proto memory:

![1st ed](screenshot.png)

## Startup system ordering

```mermaid
graph LR
    gen[generate_h_grid] --> seed[seed_ground_level]
    gen --> verify[verify_gap_counts<br/><i>debug only</i>]
    seed -- TerrainSeededPhase --> spawn[spawn_drone]
    spawn -.-> hide[hide_cursor<br/><i>native only</i>]

    subgraph h_terrain
        gen
        seed
        verify
    end

    subgraph drone
        spawn
        hide
    end
```
