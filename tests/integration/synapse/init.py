"""Initialize only the disposable m3.test homeserver volume; never print secrets."""
import os
from pathlib import Path
import secrets
import yaml

root = Path('/data')
config = root / 'homeserver.yaml'
if config.exists():
    raise SystemExit('Refusing to replace an existing fixture; use cargo xtask synapse reset')
registration = {
    'id': 'm3-m0',
    # No transaction receiver exists in M0. M1 adds the real authenticated AS path.
    'url': None,
    'as_token': secrets.token_urlsafe(32),
    'hs_token': secrets.token_urlsafe(32),
    'sender_localpart': 'm3_control',
    'namespaces': {
        'users': [{'exclusive': True, 'regex': r'@m3_[a-z0-9_]+:m3\.test'}],
        'aliases': [], 'rooms': [],
    },
    'rate_limited': False,
}
settings = {
    'server_name': 'm3.test',
    'pid_file': '/data/homeserver.pid',
    'public_baseurl': 'http://localhost:18008/',
    'listeners': [{'port': 8008, 'tls': False, 'type': 'http',
                   'bind_addresses': ['0.0.0.0'], 'x_forwarded': False,
                   'resources': [{'names': ['client'], 'compress': False}]}],
    'database': {'name': 'sqlite3', 'args': {'database': '/data/homeserver.db'}},
    'log_config': '/data/log.config',
    'media_store_path': '/data/media_store',
    'signing_key_path': '/data/m3.test.signing.key',
    'report_stats': False,
    'enable_registration': False,
    'trusted_key_servers': [],
    'suppress_key_server_warning': True,
    'app_service_config_files': ['/data/registration.yaml'],
    'macaroon_secret_key': secrets.token_urlsafe(32),
    'form_secret': secrets.token_urlsafe(32),
}
logging = {
    'version': 1,
    'handlers': {'console': {'class': 'logging.StreamHandler', 'level': 'WARNING'}},
    'root': {'handlers': ['console'], 'level': 'WARNING'},
    'disable_existing_loggers': False,
}
for name, value in [('registration.yaml', registration), ('log.config', logging), ('homeserver.yaml', settings)]:
    path = root / name
    path.write_text(yaml.safe_dump(value))
    os.chmod(path, 0o600)
    os.chown(path, 991, 991)
# Synapse runs as its packaged non-root UID after initialization.
os.chown(root, 991, 991)
print('Disposable m3.test configuration initialized; credentials remain in its volume')
