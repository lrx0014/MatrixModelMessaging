"""Dedicated local M1 fixture. This must never initialize a production server."""
import json
import os
from pathlib import Path
import runpy
import yaml

runpy.run_path('/fixtures/init.py')
root = Path('/data')
registration = yaml.safe_load((root / 'registration.yaml').read_text())
registration.update(id='m3-m1', url='http://host.docker.internal:18019', sender_localpart='_m3_control')
registration['namespaces']['users'][0]['regex'] = r'@_m3_[a-z0-9_]+:m3\.test'
settings = yaml.safe_load((root / 'homeserver.yaml').read_text())
settings.update(public_baseurl='http://localhost:18018/', enable_registration=True,
                enable_registration_without_verification=True)
# This unauthenticated registration is restricted to a disposable loopback fixture.
settings['rc_registration'] = {'per_second': 100, 'burst_count': 100}
settings['rc_message'] = {'per_second': 100, 'burst_count': 100}
for name, value in [('registration.yaml', registration), ('homeserver.yaml', settings)]:
    (root / name).write_text(yaml.safe_dump(value))
control = dict(homeserver_url='http://127.0.0.1:18018', server_name='m3.test',
               as_token=registration['as_token'], hs_token=registration['hs_token'])
path = root / 'control.json'
path.write_text(json.dumps(control))
os.chmod(path, 0o600)
os.chown(path, 991, 991)
print('M1 namespace, AS callback, and disposable human-client registration configured')
