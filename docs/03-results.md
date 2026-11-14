# Results: three rounds on a real device

> 中文版：[03-results.zh.md](03-results.zh.md)

One iPhone, iOS 26, Metal 4. Numbers are copied from `nxhost.log` on the device and from the
frame sampler; the corpus and pipeline list are the ones built into the app.

## Coverage

| Round | Recorded pipelines | Prewarmed | Missing AOT | Distinct missing vs/fs pairs |
|---|---|---|---|---|
| Baseline | 7060 | 1209 (17%) | 5851 (83%) | 2719 |
| After round 1 | 7374 | 2844 (39%) | 4530 (61%) | 2678 |
| After round 2 | 7497 | 3152 (42%) | 4345 (58%) | 2579 |

## Startup

| Round | Prewarm time | Bases from archives | Bases compiled | First frame waited |
|---|---|---|---|---|
| Baseline | 0.2 s | 503 | 0 | 0.2 s |
| After round 1 | 31.2 s | 1319 | 0 | 33.3 s |
| After round 2 | 7.0 s | 1308 | 171 | 7.5 s |

The 33.3 s was a one-time cost of precompiling bases the app had never seen; the following
launch with the same binary took 0.2 s because the device archives were complete.
Round 2 compiles 171 new bases once and writes their archives, so its second launch should be
back to ~0.2 s (not yet measured on this build).

## Worst frames

| Session | Worst frame | Breakdown |
|---|---|---|
| Baseline | 14952 ms (frame 2150) | pipeline 7577, base 7617, specialize 0, shader compile 7259 |
| Baseline, later | 7912 ms (frame 4196) | pipeline 3344, base 3359, specialize 0, shader compile 4499 |
| After round 2 | none above 100 ms | 91 samples on the title screen: worst 79 ms, one frame >50 ms, median 18 ms |

GPU time in the round-2 samples was 6.6 ms/frame, so the remaining variance is CPU-side.

## Corpus growth

| | Blobs |
|---|---|
| Build-time ExeFS scan (baseline) | 655 |
| After round 1 merge | 1841 |
| Captured on device, not yet merged | 1847 pairs (`.bin` + `.metal`) |

The 1847 pairs are the material for round 3: mostly tracks, items and effects that had never
been reached during capture.

## How the numbers were taken

- `prewarmed A/B` and `prewarm coverage` are printed by `state::prewarm`;
- the slow-frame line is printed by `submit_now` when a frame exceeds 100 ms;
- the 91-frame sample is the same counters plus GPU timestamps, sampled by a small host script
  while the app sat on the title screen;
- "worst sample 79 ms" is the worst of those 91, not a claim that no frame anywhere was slower.

## Threats to validity

- One device, one GPU family. Metal archives and even compile times are device-specific.
- Corpus coverage counts *recorded* pipelines, and the recording itself grows as you play;
  the ratio improves partly because the list gets more complete.
- Startup numbers include first-launch unpacking of the app bundle, which varies with install
  method.
