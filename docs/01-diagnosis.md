# Diagnosis: where the stalls come from

> 中文版：[01-diagnosis.zh.md](01-diagnosis.zh.md)

## The rendering model in one paragraph

The guest game issues NVN commands. `nxhost` turns them into Metal: an NVN shader blob is
translated to MSL, then compiled — at build time into the AOT corpus, or at runtime on the
render thread. A `MTLRenderPipelineState` is built from a *base* (vertex+fragment program) and
*specializations* (target formats, blend state, colour mask…). Base construction is where the
Metal compiler runs; specialization is cheap.

On iOS there is no JIT and no entitlement to spawn a process that could compile off-thread
cheaply, so anything missing from the corpus is compiled synchronously while a frame is being
submitted.

## Instrumentation

`crates/nxhost/src/nvn/draw/state.rs` keeps four process-wide nanosecond counters —
`BUILD_NS`, `BASE_NS`, `SPEC_NS`, `SHADER_NS` — fed from `build_pipeline`, `Pso4::base`,
`Pso4::pipeline` and `shaders::compile`. `submit_now` samples them around every frame and, when
a frame exceeds `NX_SLOW_FRAME` ms (default 100), prints where the time went:

```
[gfx] slow frame 2150: 14952 ms from submit to commit
  (all threads: pipeline 7577 ms, base 7617 ms, specialize 0 ms, shader compile 7259 ms)
```

The counters are process-wide on purpose: pipelines are also built by prewarm worker threads,
and a slow frame inherits their work.

## Baseline (first iOS install)

```
[gfx] prewarmed 1209/7060 recorded pipelines in 0.2 s on 6 workers
[gfx] pipeline bases after prewarm: 503 from archives, 0 compiled, 1209 specializations, 0 failed
[gfx] first frame waited 0.2 s for the pipeline prewarm
```

Only 17% of the recorded pipelines had an AOT shader; 5851 pipelines (from 2719 distinct vs/fs
pairs) had to be compiled on first use. In a race start this produced a 14.9 s frame:

- `pipeline ≈ base` → the cost is building the base program, i.e. the Metal compiler;
- `specialize ≈ 0` → specialization is not the problem;
- `shader compile ≈ 7.3 s` → even the MSL compile of that one program was seconds long.

A second session hit 7912 ms (pipeline 3344 ms, base 3359 ms, shader compile 4499 ms) on a
different pipeline. Both are content that a build-time ExeFS scan cannot see, because the
pipeline variants are assembled at runtime from state the build never executes.

## Why scanning ExeFS is not enough

`nxshader::corpus::scan` walks the game modules, unpacking yaz0 and up to four levels of SARC,
and indexes every NVN shader blob it finds (key = FNV-1a of the code section). That covers
shaders that are shipped as data. It does not cover:

- pipelines whose shader pair exists but whose *variant* was never recorded (target formats,
  extra outputs, integer targets);
- shaders generated or patched at runtime;
- code paths (tracks, modes, DLC) the branch never reached.

The coverage report added in `state::prewarm` quantifies exactly this: `missing/total recorded
pipelines have no AOT shader, from N distinct vs/fs pairs`. That number is the objective the
capture loop drives down.
