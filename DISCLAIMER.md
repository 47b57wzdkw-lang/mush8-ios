# Disclaimer

This repository and everything in [`docs/`](docs/) are published for **study, research,
interoperability analysis and technical discussion only**. Read the points below before you build,
run or quote anything from it.

## 1. Study, discussion and research only

The documentation is an engineering write-up: how a shader-compilation stall was measured, how the
device was made to report its own missing shaders, and how that feedback loop was turned into a
corpus. It exists to explain a technique, not to ship a product.

## 2. No commercial use

You may not sell, rent, license, sublicense or otherwise commercially exploit this documentation,
its translations, or any captured/merged game-derived data produced with the tooling described
here. It is not a product: there is no support, no SLA and no warranty of fitness for any purpose.

The **source code** of this repository is a fork of Mush8 and remains under GPL-3.0 (see
[`LICENSE`](LICENSE)). That licence governs the code and permits commercial use of the code; the
restriction in this section applies to the **guide and to game-derived data**, not to the code
itself.

## 3. No game data is distributed

This repository contains no `prod.keys`, no `.nsp`/`.xci`/`.nca`, no extracted ExeFS modules, no
firmware, no Mii data, no captured shaders and no Metal pipeline archives. It never will. The
capture and merge tooling only processes files the user already has on their own machine, and the
capture output (`.bin`, `.metal`, `pipelines.tsv`, `corpus.zip`, `work/pso4`) is derived from the
game and **must not be redistributed**.

## 4. You must own the game

Building or running anything described here requires your own, legally obtained copy of the game
and of the console it was dumped from. Do not use this guide to obtain, share or distribute
copyrighted content, and do not use it to circumvent technical protection measures. The authors do
not condone or assist piracy in any form.

## 5. Not affiliated with Nintendo

"Mario Kart 8 Deluxe", "Nintendo Switch" and all related names, marks and assets are trademarks
or copyrights of Nintendo and its partners. This is an independent, non-commercial interoperability
and preservation effort. It is not endorsed by, sponsored by or affiliated with Nintendo, and no
such affiliation is claimed.

## 6. Provided as is, no warranty, no liability

The documentation and tooling are provided "as is", without warranty of any kind, express or
implied, including but not limited to the warranties of merchantability, fitness for a particular
purpose and non-infringement. In no event shall the authors or contributors be liable for any
claim, damage or other liability arising from the use of this material. You are responsible for
complying with the laws of your jurisdiction and with any agreement that applies to you.

## 7. Corrections and takedown

If you believe something here infringes your rights, or you spot a factual or legal problem, open
an issue and the material will be reviewed and, where appropriate, removed promptly.
