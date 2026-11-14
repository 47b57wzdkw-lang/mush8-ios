#!/usr/bin/env python3
"""Merge runtime-captured NVN shader blobs and pipeline records into the build corpus.

  python3 tools/merge_corpus.py --repo . --capture dump_msl_rt --pipelines dump_work/pipelines_rt.tsv

The corpus is a zip of raw NVN shader blobs; entries are deduplicated by SHA-1 and written with
fixed timestamps so the archive is reproducible. Pipeline records are merged as a set.
"""
import argparse, glob, hashlib, os, zipfile


def fnv1a(b):
    h = 0x811C9DC5
    for c in b:
        h = ((h ^ c) * 0x01000193) & 0xFFFFFFFF
    return h


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--repo', default='.')
    ap.add_argument('--capture', required=True, help='directory of captured *.bin blobs')
    ap.add_argument('--pipelines', help='captured pipelines.tsv to merge')
    a = ap.parse_args()

    corpus = os.path.join(a.repo, 'data', 'corpus.zip')
    existing = {}
    if os.path.exists(corpus):
        with zipfile.ZipFile(corpus) as z:
            for n in z.namelist():
                existing[n] = z.read(n)
    print('existing corpus entries:', len(existing))

    caps, bad = {}, []
    for p in sorted(glob.glob(os.path.join(a.capture, '*.bin'))):
        raw = open(p, 'rb').read()
        if len(raw) < 0x80:
            bad.append((os.path.basename(p), len(raw)))
            continue
        caps[os.path.basename(p)] = raw
    print('captured blobs:', len(caps), 'too-small:', len(bad))

    merged, added, renamed = dict(existing), 0, 0
    for name, raw in caps.items():
        if name in merged:
            if merged[name] == raw:
                continue
            alt = name[:-4] + '_rt.bin'
            while alt in merged and merged[alt] != raw:
                alt = alt[:-4] + 'x.bin'
            merged[alt] = raw
            renamed += 1
        else:
            merged[name] = raw
            added += 1
    print('added:', added, 'collision-renamed:', renamed, 'total:', len(merged))

    seen, uniq = {}, {}
    for name, raw in merged.items():
        h = hashlib.sha1(raw).hexdigest()
        if h in seen:
            continue
        seen[h] = name
        uniq[name] = raw
    print('unique payloads:', len(uniq))

    names = sorted(uniq)
    os.makedirs(os.path.join(a.repo, 'data'), exist_ok=True)
    tmp = corpus + '.new'
    with zipfile.ZipFile(tmp, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for n in names:
            zi = zipfile.ZipInfo(n, (1980, 1, 1, 0, 0, 0))
            zi.compress_type = zipfile.ZIP_DEFLATED
            zi.external_attr = 0o644 << 16
            z.writestr(zi, uniq[n])
    os.replace(tmp, corpus)
    print('corpus.zip ->', len(names), 'entries,', os.path.getsize(corpus), 'bytes')

    if a.pipelines:
        tsv = os.path.join(a.repo, 'data', 'pipelines.tsv')
        old = open(tsv, encoding='utf-8').read().splitlines() if os.path.exists(tsv) else []
        rt = open(a.pipelines, encoding='utf-8').read().splitlines()
        seen_row, out = set(), []
        for row in old + rt:
            row = row.rstrip('\r')
            if not row.strip() or row in seen_row:
                continue
            seen_row.add(row)
            out.append(row)
        open(tsv, 'w', encoding='utf-8', newline='\n').write('\n'.join(out) + '\n')
        print('pipelines.tsv: old=%d rt=%d merged=%d' % (len(old), len(rt), len(out)))


if __name__ == '__main__':
    main()
