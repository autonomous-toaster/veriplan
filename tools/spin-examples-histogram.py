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
harness (tests/construct_verdicts.rs) does that through spin_rs::verify, and any equivalent works. A
fourth argument overrides the per-model timeout in seconds, default 120; a model that runs past it is
counted as a timeout rather than stopping the run.
"""
import collections, os, re, subprocess, sys, time

TOKEN = re.compile(r'(\d+) character\(s\) left unconsumed')


def verdict(binary, path, timeout):
    """A model that runs past the timeout is a bucket, not a crash: the first version of this script died on
    one and produced no histogram at all."""
    try:
        out = subprocess.run([binary, path], capture_output=True, text=True, timeout=timeout).stdout
    except subprocess.TimeoutExpired:
        return 'timeout', ''
    first = out.split('\n')[0]
    text = ' '.join(re.sub(r'.*string "[^"]*"[^:]*:[0-9]*: ', '', out).split())
    # A refusal carries no Lua prefix, so its text still starts with the harness wrapper. Stripping it here
    # keeps every classifier below keyed on the engine's own words rather than on our output format.
    text = re.sub(r'^spin-rs: refused \((.*)\)$', r'\1', text)
    return first, text


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
        ('_pid', 'local initialised from _pid'),
        ('declares parameters', 'proctype parameters'),
        ('provided', 'provided'),
        ('a call to', 'unknown call'),
    ]:
        if key in t:
            return label
    # A named refusal this list does not know is still a named refusal: describe it from its own words
    # rather than dropping it into an "other" bucket that hides how many distinct constructs are there.
    # The list above is a convenience, not a definition, and it has already fallen behind once.
    if 'not implemented' in t or 'not supported' in t:
        words = re.split(r'[:.]', t, maxsplit=1)[0].split()
        return 'refusal: ' + ' '.join(words[:5])
    return 'other'


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    root = sys.argv[1]
    binary = sys.argv[3] if len(sys.argv) > 3 else 'target/debug/srs-diff'
    timeout = int(sys.argv[4]) if len(sys.argv) > 4 else 120
    models = [os.path.join(d, f) for d, _, fs in os.walk(root) for f in fs if f.endswith('.pml')]
    started = time.monotonic()
    totals = collections.Counter()
    parse = collections.Counter()
    examples = {}
    for path in sorted(models):
        first, text = verdict(binary, path, timeout)
        if first == 'timeout':
            totals['timeout (model ran past the limit)'] += 1
            continue
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
    print('models:', len(models), '| timeout', timeout, 's')
    for k, n in totals.most_common():
        print('  %3d  %s' % (n, k))
    print('elapsed: %.1f s' % (time.monotonic() - started))
    if parse:
        print('\nparse-error buckets, by the source line at the offset:')
        for k, n in parse.most_common():
            print('  %3d  %-30s eg %s' % (n, k, examples[k]))
    return 0


if __name__ == '__main__':
    sys.exit(main())
