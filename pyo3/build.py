#!/usr/bin/python

import os
import subprocess


subprocess.run(["maturin", "build"], check = True)

f = sorted([f for f in os.scandir("target/wheels") if f.name.endswith(".whl")], key = os.path.getctime)[-1];

subprocess.run(["pip", "install", "--user", "--force-reinstall", f.path], check = True)
