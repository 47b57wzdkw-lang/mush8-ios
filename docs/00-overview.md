# Overview: teaching the device to report its own missing shaders

> 中文版：[00-overview.zh.md](00-overview.zh.md)

> Recorded 2026-11-14. The repository ships source and tooling only — **no game data** of any
> kind (keys, .nsp, ExeFS modules, shaders, pipeline archives).

## 1. The problem

Mush8 maps the Switch's NVN graphics API onto Metal. A guest shader has two possible outcomes:

- the build-time scan of the ExeFS modules translates it to MSL and compiles it into the AOT
  corpus;
- anything the scan cannot see is translated and compiled **on the render thread** the first
  time a scene needs it.

iOS has no JIT, so the second path is the hitch. On the first install **83% (5851 / 7060)** of
the recorded pipelines had no AOT shader. The worst frame was **14952 ms**, and the log splits it
cleanly:

```
[gfx] slow frame 2150: 14952 ms from submit to commit
  (all threads: pipeline 7577 ms, base 7617 ms, specialize 0 ms, shader compile 7259 ms)
```

`pipeline ≈ base` and `specialize ≈ 0` say the cost is compiling the base program — the Metal
compiler — not specialization. So the fix is to move compilation to first launch after install,
not to reduce specializations.

## 2. Method: let the device report what it had to compile

1. **Capture.** With `NX_DUMP_MSL=1` and `NX_RECORD_PIPELINES=1`, every shader compiled on
   device is written as an NVN blob (`.bin`) plus the MSL that was compiled (`.metal`) into
   `Documents/work/msl_rt/`; every built pipeline (vs/fs keys, targets, blend, colour state…) is
   appended to `Documents/data/pipelines.tsv`.
2. **Merge.** `tools/merge_corpus.py` deduplicates blobs by content, renames collisions, rebuilds
   `data/corpus.zip` and merges the pipeline rows.
3. **Attribute.** `nxshader` gained `scan_paths`, which walks yaz0/SARC (up to four levels) and
   records which file each shader came from — used to explain why the build-time scan missed it.
4. **Prewarm.** On the first frame six worker threads build every recorded pipeline; the log
   reports `prewarmed A/B` and the number of missing vs/fs pairs.
5. **Measure.** Four process-wide nanosecond counters (`BUILD_NS`, `BASE_NS`, `SPEC_NS`,
   `SHADER_NS`) plus a slow-frame log (`NX_SLOW_FRAME`, default 100 ms) that splits each bad
   frame into pipeline / base / specialization / shader-compile time.
6. **Tune.** `BASE_WAIT` went from 1 s to 150 ms: waiting for another thread to finish an
   identical base is only a shortcut, and a long deadline stalls the render thread for nothing.
7. **Repeat.** Play new content → capture → merge → reinstall, until coverage plateaus.

## 3. Three rounds of numbers

| Metric | Baseline | After round 1 | After round 2 |
|---|---|---|---|
| Recorded pipelines | 7060 | 7374 | 7497 |
| Prewarmed | 1209 (17%) | 2844 (39%) | 3152 (42%) |
| Missing AOT shaders | 5851 (83%) | 4530 (61%) | 4345 (58%) |
| Prewarm time | 0.2 s (all cached) | 31.2 s | 7.0 s |
| First-frame wait | 0.2 s | 33.3 s (one-off) | 7.5 s (one-off) |
| Worst frame | 14952 ms | 7912 ms | none >100 ms (worst sample 79 ms) |

- Prewarm fell from 31.2 s to 7.0 s because the device's Metal 4 pipeline archives (`work/pso4`,
  1308 bases) ship with the app; only 171 bases still had to be compiled. The next launch should
  be back to ~0.2 s.
- Title-screen sampling, 91 frames: worst 79 ms, one frame >50 ms, median 18 ms, GPU 6.6 ms per
  frame.
- Corpus: 655 blobs (build-time ExeFS scan) → 1841 (current local `data/corpus.zip`). The device
  has meanwhile accumulated 1847 more `.bin`/`.metal` pairs for round 3.

## 4. What is left

- The 58% gap is content that was never played (other tracks, items, effects). Those still
  compile on first use — occasional 100–600 ms frames. Closing it is a loop, not a switch.
- Pipeline archives and the shader corpus are derived from game data and **cannot be
  redistributed**; this repository publishes the method and the tooling only.
- Numbers come from a single device (iOS 26 / Metal 4); other GPU families need their own round.

## 5. Tools and steps (quick reference)

**Tools.** Apple Silicon Mac on macOS 14+ (26 for these numbers); Xcode 26 with the iOS 26 SDK and
the Metal Toolchain (`xcodebuild -downloadComponent MetalToolchain`); Rust 1.98.1 with the
`aarch64-apple-ios` target; Homebrew's `cmake`, `pkgconf`, `xcodegen`; Python 3.10+ with
pymobiledevice3 11.51 for pulling captures; an iPhone/iPad on iOS 26+ with 6 GB of RAM; and your
own game dump (`prod.keys`, `base.nsp`, `update.nsp` 4.0.1 or 3.0.5, optional `dlc.nsp`).

**Install.**

```sh
xcodebuild -downloadComponent MetalToolchain
brew install cmake pkgconf xcodegen
rustup toolchain install 1.98.1 && rustup target add aarch64-apple-ios --toolchain 1.98.1
python3 -m pip install --user pymobiledevice3
```

**Build.** `./make-ios.sh --ipa` (ad-hoc signed; `BUNDLE_ID`, `TEAM` and `FREE=1` are optional);
add `PSO_GPUS=applegpu_g14` to render the Metal 4 pipeline archives for your GPU family; then
`xcrun devicectl device install app --device <id> build/Mush8.ipa`.

**Capture.** Put `NX_DUMP_MSL=1`, `NX_RECORD_PIPELINES=1`, `NX_SLOW_FRAME=100`,
`NX_PREWARM_THREADS=6` and `NX_FPS=1` in the app's `Documents/env.txt`, relaunch, and play the
content once.

**Pull and merge.**

```sh
python3 tools/pull_shader_dumps.py --bundle <BUNDLE_ID> \
    --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv --log dump_work/nxhost.log
python3 tools/merge_corpus.py --repo . --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv
```

**Verify.** Rebuild, reinstall, then
`grep -E 'prewarmed|coverage|first frame waited|slow frame' dump_work/nxhost.log` and compare with
the expected shapes in [`03-results.md`](03-results.md).

The full version - every parameter, the build matrix, the CI route and troubleshooting - is in
[`06-toolchain.md`](06-toolchain.md). Every chapter also has a Chinese full version in the
matching `*.zh.md` file (see [`00-overview.zh.md`](00-overview.zh.md) §6 for the list).

## 6. Disclaimer

This guide, its translations and any game-derived data produced with it are published for **study,
discussion and research only**, and are **not licensed for commercial use**. No game data of any
kind is distributed here, and none will be: building or running anything described requires your
own copy of the game. This is an independent effort with no affiliation to Nintendo. The source
code remains under GPL-3.0. The full terms are in [`DISCLAIMER.md`](../DISCLAIMER.md)
([中文](../DISCLAIMER.zh.md)).
