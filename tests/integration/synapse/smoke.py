"""Probe public Matrix APIs against this container's disposable homeserver only."""
import json
from pathlib import Path
import urllib.error
import urllib.request
import yaml

registration = yaml.safe_load(Path('/data/registration.yaml').read_text())
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
base = 'http://127.0.0.1:8008/_matrix/client/v3'
user = '@m3_probe:m3.test'

def request(path, body=None, authorized=False, method=None):
    headers = {'Content-Type': 'application/json'}
    if authorized:
        headers['Authorization'] = 'Bearer ' + registration['as_token']
    req = urllib.request.Request(base + path, headers=headers,
        data=json.dumps(body).encode() if body is not None else None, method=method)
    try:
        with opener.open(req, timeout=10) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, json.load(error)

status, _ = request('/profile/' + user + '/displayname')
assert status == 404, 'Fixture is not fresh; reset before running smoke'
status, result = request('/register', {
    'type': 'm.login.application_service', 'username': 'm3_probe', 'inhibit_login': True,
}, authorized=True)
assert status == 200, f'AS registration failed with HTTP {status}'
assert result['user_id'] == user
status, _ = request('/profile/' + user + '/displayname?user_id=' + user,
    {'displayname': 'M0 probe'}, authorized=True, method='PUT')
assert status == 200
status, result = request('/profile/' + user + '/displayname')
assert status == 200 and result['displayname'] == 'M0 probe'
status, _ = request('/register', {
    'type': 'm.login.application_service', 'username': 'outside_namespace', 'inhibit_login': True,
}, authorized=True)
assert status == 400, f'AS namespace boundary returned unexpected HTTP {status}'
print('Fresh Synapse: AS virtual account and profile APIs passed; namespace boundary enforced')
