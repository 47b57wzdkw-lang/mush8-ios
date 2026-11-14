# Publishing this repository: the flow, and every place it broke

> 中文版：[07-publish.zh.md](07-publish.zh.md)

How these notes and the iOS port source were pushed to GitHub from a **Windows** host on
2026-11-14, including the eleven failures along the way and the exact fix for each. Everything
below is reproducible with a normal GitHub account; no game data is involved.

> Companion chapters: [`06-toolchain.md`](06-toolchain.md) covers the build/capture/merge
> toolchain and its own troubleshooting table. This file covers **publishing only**.

## A. What you need

| Piece | Version used | Notes |
|---|---|---|
| Windows host | Windows 11 2H2 | any recent Windows works |
| Git for Windows | 2.x (`git version`) | ships its own libcurl/c-ares |
| PowerShell | 7.x (`pwsh`) | used for all REST calls |
| GitHub CLI | 2.104.0 (`winget install GitHub.cli`) | optional; see problem P4 |
| Local HTTP proxy | `127.0.0.1:7890` (Clash) | only if your network needs one |
| GitHub account | `47b57wzdkw-lang` | the account that owns the repo |

The repo itself is plain `git`; nothing here needs Xcode or a Mac.

## B. The flow in six steps

```text
1. check what would be committed   git status --porcelain ; git ls-files | wc -l
2. create the empty remote        POST /user/repos  (name, private=false)
3. obtain a token                 OAuth device flow, or reuse an existing one
4. push                           git push <token-url> main:main
5. verify                         git ls-remote  ==  git rev-parse HEAD
6. verify rendering + metadata    API /repos, raw README.md
```

Real numbers for this repo: **320 tracked files**, **6 commits**, HEAD
`e9970139dd312d7caff0da5c506d8fe9af2f7a9a`, default branch `main`.

### 1. Check the working tree first

```sh
git -C <repo> status --porcelain    # must be empty (or only knowingly ignored files)
git -C <repo> ls-files | wc -l      # 320
git -C <repo> log --oneline -6
```

The repository ships **no** `prod.keys`, `.nsp`, extracted ExeFS modules, captured shaders or
pipeline archives. All of that is in `.gitignore`. Confirm with:

```sh
git -C <repo> ls-files | grep -Ei 'nsp|prod\.keys|exefs|\.zip$' # must print nothing
```

### 2. Create the empty repository

API route (what was actually used):

```powershell
$h = @{ Authorization = "token $TOKEN"; Accept = 'application/vnd.github+json'; 'User-Agent' = 'notes' }
Invoke-RestMethod -Method Post -Uri 'https://api.github.com/user/repos' -Headers $h `
  -Body (@{ name='mush8-ios'; private=$false } | ConvertTo-Json -Compress) -ContentType 'application/json'
```

CLI route, when the token has the scopes `gh` wants (see P4):

```sh
gh repo create <user>/mush8-ios --public --source=<repo> --remote=origin --push
```

### 3. Obtain a token (device flow)

Request a code:

```powershell
$r = Invoke-RestMethod -Method Post -Uri 'https://github.com/login/device/code' `
  -Headers @{ Accept='application/json' } `
  -Body @{ client_id='178c6fc778ccc68e1d6a'; scope='repo workflow read:org' }
# $r.user_code, $r.verification_uri, $r.interval, $r.expires_in  (899 s)
```

`178c6fc778ccc68e1d6a` is the public GitHub CLI OAuth app id; use your own app id for anything
long-lived. Print `user_code`, have the user open <https://github.com/login/device> and type it,
then poll:

```powershell
Invoke-RestMethod -Method Post -Uri 'https://github.com/login/oauth/access_token' `
  -Headers @{ Accept='application/json' } `
  -Body @{ client_id='178c6fc778ccc68e1d6a'; device_code=$r.device_code; `
           grant_type='urn:ietf:params:oauth:grant-type:device_code' }
```

`authorization_pending` → keep waiting; `slow_down` → increase the interval; `expired_token` →
request a new code (old one is dead, stop the old poller first).

### 4. Push

```sh
git -c http.proxy=http://127.0.0.1:7890 -c credential.helper= \
    push https://x-access-token:$TOKEN@github.com/<user>/mush8-ios.git main:main
```

- `-c credential.helper=` suppresses the Windows credential-manager prompt (it fails headless
  with `failed to execute prompt script (exit code 1)`).
- The token goes in the **command line only** — never in `.git/config`, never committed.
- `http.proxy` is needed only on networks like the one below (P2).

### 5. Verify the remote matches

```sh
git rev-parse HEAD          # e9970139dd312d7caff0da5c506d8fe9af2f7a9a
git ls-remote <token-url>   # HEAD and refs/heads/main must print the same hash
```

`/repos/<user>/<repo>` reports `visibility=public`, `default_branch=main`; `size` stays `0` for a
few seconds while GitHub indexes, that is normal.

### 6. Verify the rendered result

```sh
curl -s https://raw.githubusercontent.com/<user>/mush8-ios/main/README.md | wc -c  # 10301
```

Then in the browser: the README renders, the language links resolve, and the two disclaimer
files are reachable. Optionally set description/topics:

```powershell
Invoke-RestMethod -Method Patch -Uri 'https://api.github.com/repos/<user>/mush8-ios' -Headers $h `
  -Body (@{ description='Study notes: …'; topics=@('ios','metal','reverse-engineering') } | ConvertTo-Json -Compress) `
  -ContentType 'application/json'
```

## C. The eleven problems, and what actually fixed them

| # | Symptom | Cause | Fix |
|---|---|---|---|
| P1 | `gh: 术语 'gh' 不会被识别` | winget-added directory not in the running PATH | call `C:/Program Files/GitHub CLI/gh.exe` by full path, or open a new shell |
| P2 | `fatal: unable to access …: Could not resolve host: github.com` | Git's c-ares resolver ignores the Windows DNS path that .NET uses | route through the local proxy: `git -c http.proxy=http://127.0.0.1:7890` |
| P3 | `gh: error connecting to api.github.com` while PowerShell REST works | a proxy is configured in WinINET (`ProxyEnable=1, 127.0.0.1:7890`) but `HTTP(S)_PROXY` is empty and WinHTTP is "direct" | export `HTTPS_PROXY`/`HTTP_PROXY` for gh; pass `-c http.proxy` to git |
| P4 | `error validating token: missing required scope 'read:org'` | device flow asked for `scope=repo` only; `gh auth login` insists on `read:org` | either add `read:org` to the device-flow scope, or skip gh and use REST + token-url push |
| P5 | `refusing to allow an OAuth App to create or update workflow … without 'workflow' scope` | the repo contains `.github/workflows/build-ipa.yml` | token must carry `workflow`; re-authorize with `scope=repo workflow read:org`, or reuse an existing `repo, workflow` token (checked via the `x-oauth-scopes` response header) |
| P6 | `expired_token` after ~15 min | device codes live 899 s | request a new code; kill the stale poller so two loops don't fight |
| P7 | `failed to execute prompt script (exit code 1)` / `could not read Username` | no credential helper reachable from the sandboxed shell | `git -c credential.helper=` + token URL for that one push |
| P8 | `$env:...` silently empty inside `powershell -Command "..."` | bash expanded `$var` before PowerShell saw it | write a `.ps1` and run `pwsh -NoProfile -File script.ps1` |
| P9 | an untracked `CON` file appears in `git status` | Windows reserved device name created by a redirect | never `git add -A` blindly; it cannot be committed anyway |
| P10 | push succeeded but `size=0 KB` on the API | GitHub indexes asynchronously | wait ~10 s, then re-read `/repos` |
| P11 | `gh auth setup-git` still left git unauthenticated | setup only writes a helper for hosts gh is logged into; P4 meant it was not | configure auth per push (P7) instead of relying on global helper |

### The two non-obvious ones, in full

**P2/P3 — two different "no network" failures.** In one shell `curl`/`nslookup` could not resolve
`github.com`, yet `[System.Net.Dns]::GetHostAddresses('github.com')` returned `20.247.166.64` and
`Invoke-RestMethod` reached the API. Git for Windows resolves names through its own c-ares path, so
it failed exactly like `curl`. The machine had a WinINET proxy (`127.0.0.1:7890`) that .NET used
implicitly but `git`/`gh` do not read. Pointing each tool at the proxy fixed both instantly.

**P5 — scope escalation is per-file-type, not per-repo.** A token with `repo` can push 320 files,
create the repository and set metadata, but the single workflow file is gated behind `workflow`.
GitHub rejects the **whole push** (nothing lands) with a message that names the file. Two honest
options: re-authorize with the extra scope, or use a token that already has it — verify first:

```powershell
(Invoke-WebRequest -Uri 'https://api.github.com/user' -Headers @{ Authorization="token $T" }).Headers['x-oauth-scopes']
```

## D. Hygiene checklist

- [x] token used from the shell only; no `token@` URL left in `git remote -v`
- [x] `git remote -v` shows a clean `https://github.com/<user>/<repo>.git`
- [x] temporary helper scripts and the token file deleted after the push
- [x] `git status --porcelain` empty; no `.nsp`/`keys`/captures tracked
- [x] direct push only to `main` for the first upload, then normal PRs
- [ ] if a token is ever pasted into a chat or a log, revoke it at
      <https://github.com/settings/tokens> and re-issue (do not "just delete the file")

## E. Reproduce the publish from scratch (short version)

```powershell
$repo = 'D:\path\mush8-ios'
git -C $repo status --porcelain                     # empty
$h = @{ Authorization = "token $TOKEN"; Accept = 'application/vnd.github+json'; 'User-Agent' = 'notes' }
Invoke-RestMethod -Method Post -Uri 'https://api.github.com/user/repos' -Headers $h `
  -Body (@{ name='mush8-ios'; private=$false } | ConvertTo-Json -Compress) -ContentType 'application/json'
git -c http.proxy=http://127.0.0.1:7890 -c credential.helper= `
    -C $repo push https://x-access-token:$TOKEN@github.com/<user>/mush8-ios.git main:main
git -C $repo rev-parse HEAD
git -C $repo -c http.proxy=http://127.0.0.1:7890 ls-remote https://x-access-token:$TOKEN@github.com/<user>/mush8-ios.git
```

If any step fails, the table in section C maps the message to its fix.

---

*Study and research use only; see [`DISCLAIMER.md`](../DISCLAIMER.md) and
[`DISCLAIMER.zh.md`](../DISCLAIMER.zh.md). No game data is distributed.*
