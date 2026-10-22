"""Build the Windows portable ZIP from explicit, verified binary inputs."""
import argparse, hashlib, json, re, shutil, zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RETAIL_SHA = 'b8c588e46ebadbe37feca7bdf2da0ae63a8538099d14a53ec0c2241b086e5ffa'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def package(args):
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]*', args.version): raise ValueError('Invalid version')
    if sha(args.binary) != args.expected_binary_sha256: raise ValueError('ASI checksum mismatch')
    destination = args.output.resolve()/f'FFRS-UltrawideFix-{args.version}-win64-portable'
    if destination.exists(): raise ValueError('Use a new output directory; package staging already exists')
    destination.mkdir(parents=True)
    for source in (ROOT/'portable').iterdir():
        if source.is_file():
            target = destination/source.name
            if source.suffix in ('.cmd', '.ps1'):
                target.write_bytes(source.read_text(encoding='utf8').replace('\r\n','\n').replace('\n','\r\n').encode('utf8'))
            else: shutil.copy2(source,target)
    payload = destination/'payload'
    (payload/'scripts').mkdir(parents=True)
    shutil.copy2(args.binary,payload/'scripts/final_fantasy_resonance_fix.asi')
    shutil.copy2(args.loader,payload/'winmm.dll')
    config = (ROOT/'final_fantasy_resonance_fix.toml').read_text(encoding='utf8')
    config = config.replace('[hud]\nenable = false','[hud]\nenable = true')
    (payload/'scripts/final_fantasy_resonance_fix.toml').write_text(config,encoding='utf8',newline='\n')
    for source,name in [(ROOT/'LICENSE','LICENSE'),(ROOT/'LICENSES','LICENSES'),(args.loader_license,'LICENSE-UAL.txt')]:
        shutil.copy2(source,destination/name)
    metadata = {'version':args.version,'platform':'windows-x86_64','steamBuild':'24250053',
                'retailExeSha256':RETAIL_SHA,'binarySourceCommit':args.binary_source,
                'files':{str(p.relative_to(payload)).replace('\\','/'):sha(p) for p in sorted(payload.rglob('*')) if p.is_file()}}
    (destination/'package.json').write_text(json.dumps(metadata,indent=2)+'\n',encoding='utf8')
    hashes = '\n'.join(f'{sha(p)}  {p.relative_to(destination).as_posix()}' for p in sorted(destination.rglob('*')) if p.is_file())+'\n'
    (destination/'SHA256SUMS.txt').write_text(hashes,encoding='ascii')
    archive = args.output.resolve()/(destination.name+'.zip')
    with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
        for path in sorted(destination.rglob('*')):
            if path.is_file(): z.write(path,path.relative_to(destination).as_posix())
    sidecar = archive.with_name(archive.name+'.sha256')
    sidecar.write_text(f'{sha(archive)}  {archive.name}\n',encoding='ascii')
    print(json.dumps({'archive':str(archive),'sha256':sha(archive),'files':len(list(destination.rglob('*')))},indent=2))

if __name__=='__main__':
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--version',required=True)
    p.add_argument('--binary',required=True,type=Path)
    p.add_argument('--expected-binary-sha256',required=True)
    p.add_argument('--binary-source',required=True)
    p.add_argument('--loader',required=True,type=Path)
    p.add_argument('--loader-license',required=True,type=Path)
    p.add_argument('--output',required=True,type=Path)
    package(p.parse_args())
