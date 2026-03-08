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
    generate_h_grid --> seed_ground_level
    generate_h_grid --> verify_gap_counts
    seed_ground_level -- TerrainSeededPhase --> spawn_drone
    create_drone_materials --> spawn_drone

    subgraph h_terrain
        generate_h_grid
        seed_ground_level
        verify_gap_counts[verify_gap_counts<br/><i>debug only</i>]
    end

    subgraph drone
        create_drone_materials
        spawn_drone
        hide_cursor[hide_cursor<br/><i>native only</i>]
    end
```
