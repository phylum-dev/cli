#!/usr/bin/env python

import json
import sys

from subprocess import Popen, PIPE


def usage():
    print(f"usage: {sys.argv[0]} <package-lock.json>")
    sys.exit()

def call_cli(package_list):
    pkg_info = '\n'.join([f"{p[0]}:{p[1]}" for p in package_list])

    proc = Popen('phylum-cli batch -t npm'.split(), stdin=PIPE)
    out = proc.communicate(input=pkg_info.encode())
    sys.exit(proc.returncode)


if __name__ == '__main__':
    if len(sys.argv) > 2:
        usage()
    elif len(sys.argv) == 2:
        lockfile = sys.argv[1]
    else:
        lockfile = 'package-lock.json'

    with open(lockfile, 'rt') as fp:
        pkg_data = json.load(fp)

    pkg_list = []

    for key, val in pkg_data['dependencies'].items():
        pkg_list.append((key, val['version']))

    print(f"Submitting request for {len(pkg_list)} packages =>")
    call_cli(pkg_list)

