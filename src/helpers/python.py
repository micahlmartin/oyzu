"""Trusted adapter helper. No project code executes during wheel resolution.

The acquisition process runs with Docker networking disabled. A private file
channel mediates GET requests; only the host broker can contact approved sources.
The same helper provides offline install/build/report operations after freeze.
"""
import email
from html.parser import HTMLParser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import threading
import time
import tomllib
import urllib.parse
import uuid
import zipfile


def run(args, **kwargs):
    subprocess.run(args, check=True, **kwargs)


def project():
    path = Path('pyproject.toml')
    return tomllib.loads(path.read_text()) if path.exists() else {}


def requirement_lines():
    """Keep native requirement constraints/hashes; disallow source-route overrides."""
    data = project()
    from pip._vendor.packaging.requirements import Requirement
    inputs = []
    purposes = {}
    def add(value, purpose):
        parsed = Requirement(value)
        if parsed.url:
            raise ValueError('Direct URL requirements need an approved source adapter')
        inputs.append(value)
        purposes[re.sub(r'[-_.]+','-',parsed.name).lower()] = purpose
    for item in data.get('project',{}).get('dependencies',[]):
        add(item,'runtime')
    for item in data.get('build-system',{}).get('requires',['setuptools==80.9.0','wheel==0.45.1']):
        add(item,'build')
    # Build frontends and conventional pytest integrations are builder inputs.
    for item in data.get('dependency-groups',{}).get('dev',[]):
        if not isinstance(item,str):
            raise ValueError('Included dependency groups are not supported yet')
        add(item,'test')
    defaults=['build==1.2.2.post1','wheel==0.45.1']
    if Path('tests').is_dir() or Path('test').is_dir():
        defaults += ['pytest==8.3.5','pytest-cov==6.0.0']
    for item in defaults:
        name=Requirement(item).name
        if name not in purposes:
            add(item,'test' if name.startswith('pytest') else 'build')
    if 'ruff' in data.get('tool',{}):
        add('ruff==0.11.13','test')
    requirements = Path('requirements.txt')
    if requirements.exists():
        text = requirements.read_text().replace('\\\n',' ')
        for line in text.splitlines():
            line=line.strip()
            if not line or line.startswith('#') or line == '--require-hashes':
                continue
            if line.startswith('-') or ' --' in line.replace(' --hash=sha256:',' HASH:'):
                raise ValueError('Unsupported requirements directive; no alternate source fallback')
            declaration = line.split(' --hash=',1)[0]
            add(declaration,'runtime')
        # Native pip independently validates source-owned hashes in a second pass.
    return inputs, purposes


class Links(HTMLParser):
    def __init__(self, base):
        super().__init__(convert_charrefs=True)
        self.base=base
        self.links=[]
    def handle_starttag(self, tag, attrs):
        if tag != 'a':
            return
        values=dict(attrs)
        if 'href' not in values:
            return
        import html
        url=urllib.parse.urljoin(self.base,values['href'])
        parts=urllib.parse.urlsplit(url)
        base=urllib.parse.urlunsplit(parts._replace(fragment=''))
        parent,filename=base.rsplit('/',1)
        rewritten='/fetch/'+urllib.parse.quote(parent,safe='')+'/'+urllib.parse.quote(filename,safe='')
        if parts.fragment:
            rewritten += '#'+parts.fragment
        kept={key:value for key,value in values.items() if key in ['data-requires-python','data-yanked','data-dist-info-metadata','data-core-metadata']}
        extra=''.join(' '+html.escape(k)+'="'+html.escape(v or '')+'"' for k,v in kept.items())
        self.links.append('<a href="'+html.escape(rewritten)+'"'+extra+'>'+html.escape(urllib.parse.unquote(url.split('/')[-1].split('#')[0]))+'</a>')


class Bridge(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        if self.path.startswith('/index/'):
            url='https://pypi.org/simple/'+self.path[len('/index/'):]
        elif self.path.startswith('/fetch/'):
            url=urllib.parse.unquote(self.path[len('/fetch/'):])
        else:
            self.send_error(403)
            return
        # pip appends .metadata to the rewritten wheel URL; unwrap before forwarding.
        request_id=uuid.uuid4().hex
        root=Path('/broker')
        temporary=root/(request_id+'.pending')
        temporary.write_text(json.dumps({'url':url}))
        temporary.rename(root/(request_id+'.request'))
        response=root/(request_id+'.response')
        deadline=time.monotonic()+55
        while not response.exists():
            if time.monotonic()>deadline:
                self.send_error(504)
                return
            time.sleep(.01)
        info=json.loads(response.read_text())
        body=(root/(request_id+'.body')).read_bytes()
        if 'text/html' in info['contentType'] and info['status']==200:
            parser=Links(url)
            parser.feed(body.decode('utf-8'))
            body=('<!doctype html><html><body>'+''.join(parser.links)+'</body></html>').encode()
        self.send_response(info['status'])
        self.send_header('Content-Type',info['contentType'])
        self.send_header('Content-Length',str(len(body)))
        self.end_headers()
        self.wfile.write(body)
        response.unlink()
        (root/(request_id+'.body')).unlink()


def acquire():
    requirements,purposes=requirement_lines()
    manager=os.environ.get('OYZU_PYTHON_MANAGER','pip')
    constraint_args=[]
    if manager=='uv':
        lock=tomllib.loads(Path('uv.lock').read_text())
        for package in lock.get('package',[]):
            registry=package.get('source',{}).get('registry')
            if registry and registry.rstrip('/')!='https://pypi.org/simple':
                raise ValueError('uv lock references an unconfigured source')
        run(['uv','export','--locked','--offline','--no-python-downloads','--no-managed-python','--python',sys.executable,'--no-emit-project','--format','requirements-txt','--output-file','/out/uv-export.txt'],stdout=subprocess.DEVNULL)
        from pip._vendor.packaging.requirements import Requirement
        constraints=[]
        for line in Path('/out/uv-export.txt').read_text().replace('\\\n',' ').splitlines():
            line=line.strip()
            if not line or line.startswith('#'):
                continue
            declaration=line.split(' --hash=',1)[0].strip()
            parsed=Requirement(declaration)
            if parsed.url or parsed.extras:
                raise ValueError('Unsupported uv locked source form')
            constraints.append(declaration)
        Path('/out/constraints.txt').write_text('\n'.join(constraints)+'\n')
        constraint_args=['-c','/out/constraints.txt']
    server=ThreadingHTTPServer(('127.0.0.1',0),Bridge)
    threading.Thread(target=server.serve_forever,daemon=True).start()
    index='http://127.0.0.1:'+str(server.server_port)+'/index/'
    Path('/out/wheels').mkdir()
    args=[sys.executable,'-I','-m','pip','--isolated','download','--only-binary=:all:','--no-cache-dir','--disable-pip-version-check','--dest','/out/wheels','--index-url',index,'--trusted-host','127.0.0.1']
    run(args+constraint_args+requirements)
    if manager=='uv':
        run(args+['--no-deps','-r','/out/uv-export.txt'])
    if Path('requirements.txt').exists():
        run(args+['--no-deps','-r','requirements.txt'])
    server.shutdown()
    inventory(purposes,requirements)


def wheel_metadata(path):
    """Read the distribution's metadata, excluding vendored distributions."""
    from pip._vendor.packaging.utils import canonicalize_name, parse_wheel_filename
    from pip._vendor.packaging.version import Version
    expected_name, expected_version, _, _ = parse_wheel_filename(path.name)
    with zipfile.ZipFile(path) as wheel:
        metadata = [i for i in wheel.infolist()
                    if i.filename.count('/') == 1 and i.filename.endswith('.dist-info/METADATA')]
        if len(metadata) != 1 or metadata[0].file_size > 4*1024*1024:
            raise ValueError('Invalid top-level wheel metadata: ' + path.name)
        directory = metadata[0].filename.split('/')[0][:-len('.dist-info')]
        name, separator, version = directory.rpartition('-')
        if not separator or canonicalize_name(name) != expected_name or Version(version) != expected_version:
            raise ValueError('Wheel metadata directory does not match filename: ' + path.name)
        info = email.message_from_bytes(wheel.read(metadata[0]))
    if (len(info.get_all('Name', [])) != 1 or len(info.get_all('Version', [])) != 1
            or canonicalize_name(info['Name']) != expected_name or Version(info['Version']) != expected_version):
        raise ValueError('Wheel metadata identity does not match filename: ' + path.name)
    return info


def inventory(purposes, roots):
    import pip
    from pip._vendor.packaging.requirements import Requirement
    from pip._vendor.packaging.markers import default_environment
    from pip._vendor.packaging.utils import canonicalize_name
    packages={}
    requirements={}
    extras={}
    for value in roots:
        item=Requirement(value)
        extras.setdefault(canonicalize_name(item.name),set()).update(item.extras)
    for path in sorted(Path('/out/wheels').glob('*.whl')):
        info=wheel_metadata(path)
        name=canonicalize_name(info['Name'])
        if name in packages:
            raise ValueError('Multiple resolved versions for '+name)
        packages[name]={'id':name+'/'+info['Version'],'name':name,'version':info['Version'],'file':path.name,'purpose':purposes.get(name,'runtime'),'dependencies':[]}
        requirements[name]=[Requirement(r) for r in info.get_all('Requires-Dist',[])]
    changed=True
    while changed:
        changed=False
        for name,items in requirements.items():
            for item in items:
                contexts=extras.get(name,set())|{''}
                if item.marker and not any(item.marker.evaluate({**default_environment(),'extra':extra}) for extra in contexts):
                    continue
                dep=canonicalize_name(item.name)
                if dep not in packages or not item.specifier.contains(packages[dep]['version'],prereleases=True):
                    raise ValueError('Incomplete native dependency closure: '+dep)
                if item.url:
                    raise ValueError('Uncaptured direct dependency URL')
                if packages[dep]['id'] not in packages[name]['dependencies']:
                    packages[name]['dependencies'].append(packages[dep]['id'])
                active=extras.setdefault(dep,set())
                if not item.extras<=active:
                    active.update(item.extras)
                    changed=True
    for package in packages.values():
        package['dependencies'].sort()
    manager_version=subprocess.check_output(['uv','--version'],text=True).strip().split()[1] if os.environ.get('OYZU_PYTHON_MANAGER')=='uv' else pip.__version__
    Path('/out/packages.json').write_text(json.dumps({'packages':list(packages.values()),'python':sys.version.split()[0],'pip':pip.__version__,'managerVersion':manager_version},sort_keys=True))


def prepare():
    import venv
    data=project()
    if not data.get('project',{}).get('version') or not data['project'].get('name'):
        raise ValueError('Static PEP 621 name/version required for this Python build profile')
    text=Path('pyproject.toml').read_text()
    section=re.search(r'(?ms)^\[project\]\s*\n(.*?)(?=^\[|\Z)',text)
    if section is None:
        raise ValueError('Missing project table')
    content,count=re.subn(r'(?m)^(\s*version\s*=\s*)["\'][^"\']+["\']',lambda m:m[1]+json.dumps(os.environ['OYZU_VERSION']),section[1],count=1)
    if count!=1:
        raise ValueError('Cannot project snapshot version')
    Path('pyproject.toml').write_text(text[:section.start(1)]+content+text[section.end(1):])
    venv.EnvBuilder(with_pip=False).create('.oyzu-build/venv')
    wheels=sorted(str(p) for p in Path('/dependencies/wheels').glob('*.whl'))
    run([sys.executable,'-I','-m','pip','--isolated','--python','.oyzu-build/venv','install','--no-index','--no-deps',*wheels])


def build():
    python='.oyzu-build/venv/bin/python'
    if os.environ.get('OYZU_PYTHON_MANAGER')=='uv':
        run(['uv','build','--offline','--no-python-downloads','--no-managed-python','--python',python,'--no-build-isolation','--out-dir','.oyzu-build/dist'])
    else:
        run([python,'-I','-m','build','--no-isolation','--outdir','.oyzu-build/dist'])
    wheels=list(Path('.oyzu-build/dist').glob('*.whl'))
    if len(wheels)!=1:
        raise ValueError('Expected one project wheel')
    run([sys.executable,'-I','-m','pip','--isolated','--python','.oyzu-build/venv','install','--no-index','--no-deps','--force-reinstall',str(wheels[0])])


def package():
    import gzip
    import shutil
    import tarfile
    destination=Path('/out')/os.environ['OYZU_TARGET']/'artifacts'
    for artifact in Path('.oyzu-build/dist').iterdir():
        if artifact.name.endswith('.tar.gz'):
            # Normalize transport metadata; retain native backend's content and paths.
            with tarfile.open(artifact,'r:gz') as archive, (destination/artifact.name).open('wb') as raw:
                with gzip.GzipFile(filename='',mode='wb',fileobj=raw,mtime=0) as compressed:
                    with tarfile.open(fileobj=compressed,mode='w|',format=tarfile.PAX_FORMAT) as output:
                        total=0
                        for i,member in enumerate(archive):
                            if i>100000 or member.name.startswith('/') or '..' in Path(member.name).parts or not (member.isfile() or member.isdir()):
                                raise ValueError('Unsafe source archive entry')
                            total += member.size
                            if total>512*1024*1024:
                                raise ValueError('Source archive limit exceeded')
                            member.uid=member.gid=0
                            member.uname=member.gname=''
                            member.mtime=0
                            member.pax_headers={}
                            output.addfile(member,archive.extractfile(member) if member.isfile() else None)
        elif artifact.name.endswith('.whl'):
            shutil.copyfile(artifact,destination/artifact.name)
        else:
            raise ValueError('Unexpected native Python artifact')


if __name__=='__main__':
    if sys.argv[1]=='acquire':
        acquire()
    elif sys.argv[1]=='prepare':
        prepare()
    elif sys.argv[1]=='build':
        build()
    elif sys.argv[1]=='package':
        package()
    else:
        raise SystemExit('Unknown adapter operation')
