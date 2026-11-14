# Overview: teaching the device to report its own missing shaders

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
