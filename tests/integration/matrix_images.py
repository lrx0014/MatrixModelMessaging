"""Run real encrypted-client scenarios in one final M1 image architecture.

Control provisions only disposable AS accounts on the host. The container gets
user sessions and SDK/application volumes; it never receives global AS tokens.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--arch', choices=['arm64', 'amd64'], required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
arch = args.arch
base = root / '.local' / 'm1'
sessions = base / ('image-' + arch) / 'sessions'
image = 'm3-agent:m1-' + arch
volume = 'm3-m1-probe-' + arch


def run(command, **kwargs):
    return subprocess.run(command, cwd=root, check=True, **kwargs)


exists = subprocess.run(['docker', 'volume', 'inspect', volume], cwd=root,
                        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0
if sessions.exists() and any(sessions.iterdir()) and not exists:
    raise SystemExit('Refusing to reuse a device token without its SDK volume; archive this fixture and provision new devices')
sessions.mkdir(mode=0o700, parents=True, exist_ok=True)
os.chmod(sessions, 0o700)
for name in ['alice', 'bob']:
    path = sessions / (name + '.json')
    if not path.exists():
        run([str(root / 'target/debug/m3-control'), 'provision', '--config', str(base / 'control.json'),
             '--database', str(base / 'control.db'), '--localpart', '_m3_img_' + arch + '_' + name,
             '--session-output', str(path)])
run(['docker', 'volume', 'create', volume], stdout=subprocess.DEVNULL)
# The root-only initialization container is offline and mounts user sessions only.
# Never replace credentials in an existing SDK volume.
run(['docker', 'run', '--rm', '--platform', 'linux/' + arch, '--network', 'none', '--user', '0:0',
     '--mount', 'type=volume,src=' + volume + ',dst=/data',
     '--mount', 'type=bind,src=' + str(sessions) + ',dst=/provision,readonly',
     '--entrypoint', 'sh', image, '-ec',
     'mkdir -p /data/sessions /data/agents; '
     'for name in alice bob; do '
     'if [ -f /data/sessions/$name.json ]; then cmp /provision/$name.json /data/sessions/$name.json; '
     'else cp /provision/$name.json /data/sessions/$name.json; fi; done; '
     'chmod 700 /data /data/sessions /data/agents; chmod 600 /data/sessions/*.json; '
     'chown -R 65532:65532 /data'])
identity = run(['docker', 'image', 'inspect', image, '--format', '{{.Id}}'],
               capture_output=True, text=True).stdout.strip()
with (base / ('image-' + arch + '.log')).open('w') as log:
    run(['docker', 'run', '--rm', '--platform', 'linux/' + arch, image, 'storage-smoke'],
        stdout=log, stderr=subprocess.STDOUT)
    run(['docker', 'run', '--rm', '--platform', 'linux/' + arch, '--network', 'm3-m1_default',
         '--mount', 'type=volume,src=' + volume + ',dst=/data', image, 'protocol-smoke',
         '--homeserver', 'http://synapse:8008', '--sessions', '/data/sessions', '--state', '/data/agents'],
        stdout=log, stderr=subprocess.STDOUT)
(base / ('image-' + arch + '-result.json')).write_text(json.dumps({
    'architecture': arch, 'image': identity, 'result': 'passed',
    'execution': 'native Docker Linux VM' if arch == 'arm64' else 'Docker Desktop amd64 emulation',
}, indent=2) + '\n')
print(arch + ': final image SDK/SQLx/FTS5, encrypted messaging, SAS, restart, delayed keys, and crash windows passed')
