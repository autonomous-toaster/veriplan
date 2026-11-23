#!/usr/bin/env python3
"""Bucket spin-rs' refusals over spin's own shipped examples.

Two things this does that a first pass gets wrong, both learned from doing it wrong:

  * a parse error reports the offset where the *item* starts, so bucketing on the token after
    "near '...'" names the construct after the offending one. The bucket here is the source line at
    the offset, computed as length - "N characters left unconsumed".
  * a crash and a refusal are different failures and must not share a bucket. Anything whose message
    mentions a nil global is counted separately: it is an internal error, not a decision about a
    construct.

Usage:
    tools/spin-examples-histogram.py /path/to/spin/Examples [--binary ./target/debug/... ]

Needs a small runner that takes a .pml path and prints the engine's verdict; this repo's guard
harness (tests/construct_verdicts.rs) does that through spin_rs::verify, and any equivalent works.
"""
import collections, os, re, subprocess, sys

TOKEN = re.compile(r'(\d+) character\(s\) left unconsumed')


def verdict(binary, path):
    out = subprocess.run([binary, path], capture_output=True, text=True, timeout=120).stdout
    return out.split('\n')[0], ' '.join(re.sub(r'.*string "[^"]*"[^:]*:[0-9]*: ', '', out).split())


def family(first, text):
    t = text.lower()
    for key, label in [
        ('nil value', 'internal error (undefined global)'),
        ('run is not supported', 'run'),
        ('preprocessor', 'preprocessor'), ('#include', 'preprocessor'),
        ('#if', 'preprocessor'), ('function-like macro', 'preprocessor'),
        ('poll or eval receive', 'poll/eval receive'),
        ('random receive', 'random receive'),
        ('more than one variable', 'multi-field receive'),
        ('atomic/d_step', 'atomic with control flow'),
        ('unless', 'unless'), ('rendezvous', 'rendezvous'),
        ('timeout', 'timeout'), ('inline', 'inline call'),
        ('parse error', 'parse error'),
        ('transition evaluation failed', 'evaluation failure'),
    ]:
        if key in t:
            return label
    return 'other'


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    root = sys.argv[1]
    binary = sys.argv[3] if len(sys.argv) > 3 else 'target/debug/srs-diff'
    models = [os.path.join(d, f) for d, _, fs in os.walk(root) for f in fs if f.endswith('.pml')]
    totals = collections.Counter()
    parse = collections.Counter()
    examples = {}
    for path in sorted(models):
        first, text = verdict(binary, path)
        if 'errors=' in first and 'refused' not in first:
            totals['decided'] += 1
            continue
        fam = family(first, text)
        totals[fam] += 1
        m = TOKEN.search(text)
        if fam == 'parse error' and m:
            src = open(path, errors='ignore').read()
            idx = max(0, len(src) - int(m.group(1)))
            start = src.rfind(chr(10), 0, idx) + 1
            line = src[start:src.find(chr(10), start)].strip()
            key = ' '.join(line.split()[:2]) if line else '(blank)'
            if line.startswith('active proctype') or line.startswith('proctype'):
                body = re.search(r'proctype\s+\w+\s*\(([^)]*)\)', line)
                key = 'proctype (parameterised)' if body and body.group(1).strip() else 'proctype (plain)'
            parse[key] += 1
            examples.setdefault(key, os.path.basename(path))
    print('models:', len(models))
    for k, n in totals.most_common():
        print('  %3d  %s' % (n, k))
    if parse:
        print('\nparse-error buckets, by the source line at the offset:')
        for k, n in parse.most_common():
            print('  %3d  %-30s eg %s' % (n, k, examples[k]))
    return 0


if __name__ == '__main__':
    sys.exit(main())
