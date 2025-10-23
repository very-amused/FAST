#!/usr/bin/env python3
import os
import subprocess
from pathlib import Path
from sys import argv

src_root = Path(os.environ['MESON_PROJECT_SOURCE_ROOT'])
build_root = Path(os.environ['MESON_PROJECT_BUILD_ROOT'])
outfile = Path(argv[1])

subprocess.run(['cargo', 'build', '--release'], cwd=src_root).check_returncode()
subprocess.run(['cp',
                src_root / 'target' / 'release' / outfile.name,
                build_root / outfile]).check_returncode()
