# Reproduce

You need your own game dump (see `README.md`). This repository ships no game data, no keys and
no extracted modules, so a plain `git clone` will **not** produce a runnable app until you add
them.

## 1. Build

```sh
./build.sh                 # macOS
./make-ios.sh --ipa        # iOS, produces an .ipa (signed later)
./make-ios.sh --install    # iOS, installs straight to a connected device
```

`make-ios.sh` extracts and patches the game modules into `work/`, scans them for shaders, and
precompiles a corpus. With no `data/corpus.zip` it still precompiles every shader carried by
the modules — the documented fallback.

## 2. Turn capture on

Put these lines in the app's `Documents/env.txt` (iOS) or the environment (macOS):

```
NX_DUMP_MSL=1
NX_RECORD_PIPELINES=1
NX_SLOW_FRAME=100
NX_PREWARM_THREADS=6
```

Play the content you care about once. The device writes:

- `Documents/work/msl_rt/<hash>.bin` and `.metal` — every shader it had to compile;
- `Documents/data/pipelines.tsv` — every pipeline it built.

## 3. Pull and merge

```sh
# iOS, uses pymobiledevice3 + house_arrest
python3 tools/pull_shader_dumps.py --bundle ch.cclerc.mush8 \
    --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv

python3 tools/merge_corpus.py --repo . \
    --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv
```

`merge_corpus.py` rebuilds `data/corpus.zip` and merges `data/pipelines.tsv` in place. Both are
ignored by `.gitignore`: they are derived from game data and must not be committed.

## 4. Rebuild, reinstall, verify

```sh
./make-ios.sh --ipa && ./install-phone.sh --ipa
```

Then read the log:

```sh
adb shell  # not applicable on iOS; use the logging console or pull Documents/work/nxhost.log
grep -E 'prewarmed|coverage|first frame waited|slow frame' nxhost.log
```

Expected shape: `prewarmed` rises, `coverage` missing count falls, `first frame waited` settles
once the device archives are populated, and `slow frame` lines disappear for content you have
already played once.

## 5. Shader provenance (optional)

```sh
# which module/archive does a given key come from?
cargo run --release -p nxshader --bin shfind -- <exefs-dir> | head
```

`shfind` prints `key<TAB>origin::member` for every shader it can attribute, which is the fastest
way to see whether a missing key exists in the shipped data at all.
