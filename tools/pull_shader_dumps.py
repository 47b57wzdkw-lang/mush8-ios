#!/usr/bin/env python3
"""Pull captured shader pairs and pipeline records from an iOS device.

Uses pymobiledevice3 + house_arrest (the app container is reachable without jailbreak;
Documents/ is shared with the Files app). Re-running skips files already pulled.

  python3 tools/pull_shader_dumps.py --bundle ch.cclerc.mush8 \
      --out dump_msl_rt --pipelines-out dump_work/pipelines_rt.tsv
"""
import argparse, asyncio, os


async def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--bundle', required=True)
    ap.add_argument('--out', default='dump_msl_rt')
    ap.add_argument('--pipelines-out', default='')
    ap.add_argument('--log', default='')
    a = ap.parse_args()

    from pymobiledevice3.lockdown import create_using_usbmux
    from pymobiledevice3.services.house_arrest import HouseArrestService

    ld = await create_using_usbmux()
    svc = await HouseArrestService.create(lockdown=ld, bundle_id=a.bundle, documents_only=True)
    os.makedirs(a.out, exist_ok=True)

    for sub in ('work/msl_rt',):
        names = [n for n in sorted(await svc.listdir('Documents/' + sub)) if n.endswith(('.bin', '.metal'))]
        todo = [n for n in names if not (os.path.exists(os.path.join(a.out, n)) and os.path.getsize(os.path.join(a.out, n)) > 0)]
        print('%s: %d files, %d new' % (sub, len(names), len(todo)), flush=True)
        for i, n in enumerate(todo):
            data = await svc.get_file_contents('Documents/' + sub + '/' + n)
            open(os.path.join(a.out, n), 'wb').write(data)
            if (i + 1) % 100 == 0:
                print('  %d/%d' % (i + 1, len(todo)), flush=True)

    if a.pipelines_out:
        data = await svc.get_file_contents('Documents/data/pipelines.tsv')
        os.makedirs(os.path.dirname(a.pipelines_out) or '.', exist_ok=True)
        open(a.pipelines_out, 'wb').write(data)
        print('pipelines.tsv: %d bytes' % len(data))

    if a.log:
        data = await svc.get_file_contents('Documents/work/nxhost.log')
        open(a.log, 'wb').write(data)
        print('nxhost.log: %d bytes' % len(data))


if __name__ == '__main__':
    asyncio.run(main())
