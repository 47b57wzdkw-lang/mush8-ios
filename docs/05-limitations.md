# Limitations and open questions

> 中文版：[05-limitations.zh.md](05-limitations.zh.md)

## Coverage is content-shaped

The remaining 58% of recorded pipelines are mostly content that was never reached during
capture. They still compile on first use (observed 100–600 ms frames, rarely seconds). The loop
converges only as fast as you play new content, and a fresh install on a fresh device starts
from whatever corpus the build shipped.

## Device-specific archives

Metal 4 pipeline archives are produced by the device driver for that GPU and OS version. They
cut prewarm from 31.2 s to 7.0 s here, but cannot be redistributed and must be captured per
device family.

## The corpus itself is not distributable

`corpus.zip` and `pipelines.tsv` are derived from the game's shaders and pipeline state. This
repository therefore contains none of them and no capture output; only the hooks and the merge
tooling are public.

## Startup cost moved, not removed

Prewarm is a one-time cost per new content: 7.5 s first frame on this build, ~0.2 s once the
device archives are complete. It is bounded and visible, but it is still startup time, and on
low-RAM devices the corpus + archives compete with the game's memory.

## Single device, single game build

All numbers here come from one iPhone on iOS 26 with Metal 4 and one game build. Compile times,
archive compatibility and thread saturation will differ on other GPUs, and the corpus keys are
tied to the exact game/update version.

## Open questions

- Can the missing pairs be predicted from the pipeline state (colour mask, targets) instead of
  requiring a play session for each new variant?
- Would compiling during the loading screen (instead of at first frame) hide the 7.5 s entirely?
- How much of the remaining 58% is unreachable in normal play, versus simply unvisited?
