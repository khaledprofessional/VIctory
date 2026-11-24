# This fork: running GTA VI on a PC GPU, with the shader gaps closed

This is a fork of [Marvisak/VIctory](https://github.com/Marvisak/VIctory)
(GPL-3.0-or-later, see [LICENSE](LICENSE)) whose branch `nvidia` carries the
community GPU and shader work that was sitting in open pull requests, merged
and tested together, plus the fixes below. The point of the branch is to get
the title further on a normal PC GPU — in particular on an NVIDIA card, where
the host wave is 32 lanes and the console's compute shaders are sometimes 64 —
and to make more of the title's shaders translate instead of being dropped.

The upstream README still describes the project as a whole; nothing here
changes the game files you must supply, and no copyrighted asset is stored in
this repository.

## What is in the branch

`nvidia` is `exp/world` (the branch this fork started from) plus:

| Commit | What it does |
|--------|--------------|
| 9ab5f21 | 64-bit image atomics, `s_buffer_load` of descriptor sizes, spilled lane masks |
| merge of [#1402](https://github.com/Marvisak/VIctory/pull/1402) | the high-yield shader bundle: a descriptor read only as data takes its kind from its table, 64-bit image atomics, biased 16-bit image samples and validator fixes, packed 16-bit pixel inputs, `image_get_lod` on cubes, `image_sample_d_cl`, a select or carry-in on an untracked mask, a lane mask spilled to a VGPR lane |
| merge of [#1407](https://github.com/Marvisak/VIctory/pull/1407) | a lane mask holding lanes that do not run where it is written, and mask joins, read as numbers |
| merge of [#1412](https://github.com/Marvisak/VIctory/pull/1412) | scalar comparisons between lane masks stay uniform (a branch no longer diverges between lanes that agree) |
| merge of [#1409](https://github.com/Marvisak/VIctory/pull/1409) | heap-loaded `s_buffer_load` reads raw (a CBV handle on an SRV slot is undefined on the host), `SampleLevel` outside pixel shaders |
| `GPU: run on the high-performance adapter` | the host device is created on DXGI's high-performance adapter instead of the default one, which on a machine with an integrated GPU is often the iGPU; `VICTORY_ADAPTER=<index>` or `<part of the name>` overrides it, and every adapter is logged |

The base branch already carries the Wave64 emulation for 32-lane hosts
(console compute shaders that need 64-lane waves are emulated), the Agility
SDK opt-in, mesh pipelines loaded from the title's pipeline library, the
descriptor/structured-view fixes and the `VICTORY_TEXTURE_MIP_DROP` switch.

## What it measures

`victory translate` runs every shader it finds through the translator and
lists what it cannot handle. On a retail **shaders.rpf** (26,084 programs)
the emitted count goes:

| Stage | before | after | delta |
|-------|--------|-------|-------|
| Vertex and pixel | 12,854 of 14,027 | 13,104 of 14,027 | +250 |
| Compute | 1,465 of 2,582 | 1,982 of 2,582 | +517 |
| Mesh | 8,380 of 8,713 | 8,380 of 8,713 | 0 |
| **All** | **22,699 (87.0%)** | **23,466 (89.9%)** | **+767** |

The failure reasons that the run in the issue log shows, before → after:

| Reason | before | after |
|--------|--------|-------|
| a descriptor read only as data | 613 | 57 |
| a select on an untracked mask | (runtime only) | 0 |
| a lane mask holding lanes elsewhere read as data | 273 | 260 |
| a structured buffer of unknown stride | 182 | 186 |
| a descriptor's address or format read as data | 729 | 809 |

The last two rows are the honest part of this: more programs now reach
further into the emitter, so reasons that were hidden behind an earlier
refusal show up. Those two are the next real gaps — a program that reads a
descriptor's own address/format dwords, and a structured buffer whose stride
the container does not state (the work for it is in PRs #193/#704 and #317,
which are not in this branch yet).

Caveats: the counts are from `shaders.rpf` only (the multi-GiB archives are
skipped by the tool), and translation is not the same as correct rendering.
Nothing here has been verified frame by frame against the console.

## Running it

```powershell
# one-time: your own paths, outside the repository
@'
[paths]
game_dir = 'D:\GTA6'
# cache_dir = 'D:\vicfork-cache'
'@ | Set-Content victory.toml

cargo build --release -p victory

# the shader census (no game data needed for the tests, this needs your dump)
target\release\victory.exe translate D:\GTA6\shaders.rpf --list --jobs 12

# the title
target\release\victory.exe run D:\GTA6\game_scarlett_master.exe --execute
```

`victory config` prints the resolved paths. Logging is
`VICTORY_LOG=info` (a release build logs info and above); shader refusals are
`victory_gpu::pipeline` warnings that name the reason.

Machine this branch was built and measured on: Windows 11, Ryzen 7 7700X,
GeForce RTX 4060 Ti (and an AMD integrated GPU, which is why the adapter
selection above exists), Rust 1.98.1. `cargo test -p victory-shader
--release` passes (167 tests, 1 ignored); the GPU-backed tests need a
D3D12 device and skip on WARP.

## Licence and provenance

Everything merged keeps its original authorship in the commit graph; the PR
titles and numbers are above. GPL-3.0-or-later, as upstream.
