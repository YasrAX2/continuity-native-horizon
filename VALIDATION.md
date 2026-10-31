# Validation on Windows x64

- Rust 1.99.1, x86_64-pc-windows-gnu, portable Horizon toolchain.
- Horizon source: `51c9565411b3fad3795b8c5ad07238976df38024`.
- horizon-mixins: `4342a0bfbe71d4d14185528d64e496d6022f16a1`.
- Seven Rust core tests pass, including enumeration of all 256 neighbor masks into 47 standard CTM cases, negative repeat coordinates, malformed dimensions, property continuations, tile ranges and rotated UV axes.
- The three injections and four support modules apply through horizon-mixins.
- The patched renderer passes `cargo check`; the full native client builds in the development profile.
- Horizon's `cargo-horizon-mod check` accepts the `.hnmod` as a native-mod bundle.
- A real Vulkan client on an RTX 3060 loaded one procedural CTM rule and one emissive layer in an isolated world. The wall test showed merged texture interiors and a full-bright emissive patch at midnight. It also exposed randomized block-model UV rotations, for which texture-axis detection and a regression test were added. A rebuilt client then rendered the same wall with the stray seams removed; the emissive square remained bright at midnight.

This is smoke testing, not a comparison against Continuity's Java renderer. There is no claim of full feature parity, all-pack compatibility, or production performance. The installed proof build uses the unoptimized development profile. The source bundle lets Horizon build an optimized native version through its normal pipeline.

`tests/continuity-diagnostic.zip` contains only original procedural diagnostic textures. It replaces stone with a cyan connected surface with red borders and adds a white emissive square to dirt. It is a test pack, not a normal gameplay texture pack.

Local installation: a separate `Continuity Native` instance uses `native-continuity-0.1.0-51c9565`. Its original default texture pack was extracted from the supplied JAR and installed locally with its license; that pack is not included in this repository. Existing profiles and the original JAR were preserved.
