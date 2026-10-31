# Continuity Native for Horizon â€” experimental 0.1.0

Native Rust implementation of part of Continuity's resource-pack functionality.
It compiles into Horizon through its native mixin system; it does not execute Java.
**This is an early implementation, not a complete or verified equivalent of Continuity 3.0.1.**

Implemented:

- Standard 47-tile CTM, with all 256 neighbor patterns covered by core tests.
- Horizontal, vertical, horizontal+vertical, vertical+horizontal, top, fixed, repeat and deterministic random selection.
- `matchTiles`, `matchBlocks` with block-state property predicates, `connect=block|state|tile`, face filters and rule weight ordering.
- PNG/animated PNG sprites loaded from resource packs' `assets/*/optifine/ctm` directories.
- Block-model emissive overlays using `assets/minecraft/optifine/emissive.properties` and `suffix.emissive`.
- Neighbor reads across chunk boundaries, using Horizon's padded section data.

Missing / limitations:

- Compact CTM, overlay methods, item emissive textures and Continuity's built-in packs are not included.
- Biome/height/name restrictions, inner seams, weighted random, randomLoops, linked and symmetry properties are not implemented. Rules using them are skipped with a log warning.
- Random placement uses an original deterministic hash; it does not reproduce Java/OptiFine random placement.
- Rotated model UVs, multipass rules, `<default>` tiles and special pane geometry are not fully handled.
- Source-only alpha: graphical parity, performance and compatibility with other native mods need further testing.
- This patches private renderer functions and is pinned to one Horizon revision. Do not assume compatibility with other Nightlies.

## Install

In Horizon's Mods page, use **Add from GitHub** with this repository's URL. Install the native mod, then use **Build** and play the resulting native instance. Horizon must compile the game once. Native mods run with your user privileges.

Alternatively, unpack the source and run:

```text
horizon-launcher native-build PATH/TO/continuity-native --trust-native-code
```

Use a resource pack containing supported CTM/emissive rules. This package does not include Minecraft textures or Continuity's bundled texture packs. A procedural diagnostic pack is provided only for testing.

## Development

```text
cargo test
python tools/package.py
```

Apply the patches to the exact Horizon commit in `horizon-native.json`, with the exact horizon-mixins commit listed there:

```text
horizon-mixins apply --workspace PATH/TO/horizon --mods mixins --out PATH/TO/fresh-stage
cargo check --locked --manifest-path PATH/TO/fresh-stage/Cargo.toml -p horizon-client-render
```

On Windows use a fresh stage directory for every apply; horizon-mixins cannot atomically replace an existing stage there. Keep Cargo's target directory outside the stage.

Format specification: [OptiFine CTM properties](https://github.com/sp614x/optifine/blob/master/OptiFineDoc/doc/ctm.properties). No Continuity Java source was decompiled, copied or translated. The supplied JAR's glass artwork was observed to identify the standard tile layout; its pixels are not shipped. Horizon's MIT PNG decoder and face-lighting implementation are adapted with notices in `THIRD_PARTY_NOTICES.md`.

Created by YasrAX2's Codex agent. Not affiliated with or endorsed by Continuity, Horizon, Mojang or Microsoft.
