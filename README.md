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

## Update systems — Intro & Arming

```mermaid
graph LR
    run_intro -- "set Arming" --> arm_pipe
    arm_pipe -- "set Running" --> done([Running])

    subgraph "Intro state"
        run_intro
    end

    subgraph "Arming state"
        arm_pipe
    end

    recenter_cursor[recenter_cursor<br/><i>native only</i>]
```

## Update systems — Running

```mermaid
graph LR
    recenter_cursor --> fly
    fly --> update_ground_level
    update_ground_level --> track_player_fov
    track_player_fov --> start_fov_transitions
    start_fov_transitions --> animate_fov_transitions
    start_fov_transitions --> track_in_sight
    track_in_sight --> fire_laser

    subgraph drone
        recenter_cursor[recenter_cursor<br/><i>native only</i>]
        fly
        draw_crosshair
        fire_laser
        lock_cursor_on_click[lock_cursor_on_click<br/><i>wasm only</i>]
    end

    subgraph h_terrain
        subgraph HTerrainPhase
            update_ground_level
            track_player_fov
            start_fov_transitions
            animate_fov_transitions
            track_in_sight
        end
    end
```
