# Method: capture on device, merge offline, prewarm at launch

> 中文版：[02-method.zh.md](02-method.zh.md)

The loop is deliberately boring: the device tells us which shaders it had to compile, we add
them to the corpus, rebuild, and verify the counters move. Everything is driven by environment
variables so a normal build stays clean.

## 1. Capture (`NX_DUMP_MSL`, `NX_RECORD_PIPELINES`)

`crates/nxhost/src/nvn/draw/shaders.rs` `dump_requested` writes, for each shader that is
translated on device, a pair of files under `Documents/work/msl_rt/`:

- `<hash>.bin` — the original NVN shader blob (exactly what the corpus stores);
- `<hash>.metal` — the MSL that was compiled, for inspection.

`state::record` appends every built pipeline (vs/fs keys, targets, colour state, channel mask,
blend) to `Documents/data/pipelines.tsv`. Both paths are app-relative so the files survive app
restarts and can be pulled over AFC/house_arrest.

## 2. Merge (`tools/merge_corpus.py`)

`data/corpus.zip` is content-addressed: entries are raw NVN blobs, deduplicated by SHA-1, with
fixed timestamps so rebuilds are reproducible. If a captured blob has the same name as an
existing entry but different bytes, it is stored as `<name>_rt.bin` instead of overwriting.
`data/pipelines.tsv` rows are merged as a set (exact-line dedupe).

## 3. Attribute (`nxshader` `scan_paths`, `shfind`)

`corpus.rs` now has a single `blobs()` walker shared by the positional scan and a new
`scan_paths()` that records the origin of every shader (`module::sarc/member`, up to four SARC
levels). `crates/nxshader/src/bin/shfind.rs` exposes it as a CLI. This answers "why did the
build-time scan miss this key?" — whether it is absent, packed differently, or simply not in
any shipped archive.

## 4. Prewarm (`state::prewarm`)

On the first submitted frame `gfx` calls `state::wait_prewarm`, which builds every recorded
pipeline on `NX_PREWARM_THREADS` workers (default: available CPUs):

```
[gfx] prewarmed 3152/7497 recorded pipelines in 7.0 s on 6 workers
[gfx] prewarm coverage: 4345/7497 recorded pipelines have no AOT shader,
      from 2579 distinct vs/fs pairs; those compile on first use
```

The first frame waits for prewarm (bounded by design: `NX_PREWARM_WAIT=0` disables the wait).

## 5. Metal 4 pipeline archives (`work/pso4`, device-generated)

On a Metal 4 device the driver can serialise a compiled pipeline base into an archive file.
Capturing those (`PipelineBinaries`) and shipping them with the app lets the next launch skip
the compiler entirely: `pipeline bases after prewarm: 1308 from archives, 171 compiled` — the
reason prewarm fell from 31.2 s to 7.0 s between rounds. These archives are device/driver
specific and therefore are **not** part of this repository.

## 6. Tuning that came out of the numbers

- `BASE_WAIT` 1 s → 150 ms (`pso4.rs`): a build waits on an identical base compiled by another
  thread; waiting is only a shortcut, and a long deadline stalls the render thread for no gain.
- Slow-frame logging is capped at 30 reports, so instrumentation stays cheap during play.
- Prewarm worker count is tunable (`NX_PREWARM_THREADS`); 6 workers on the tested phone is
  where the compiler saturates without starving the game main thread.

## 7. The loop

1. Install a build with capture on.
2. Play the content you care about (tracks, modes, DLC) once.
3. Pull `msl_rt/` and `data/pipelines.tsv`, merge, rebuild, reinstall.
4. Check `prewarm coverage` fell, restart, confirm the worst frame and the first-frame wait.
5. Repeat until the coverage plateau matches the content you actually play.
