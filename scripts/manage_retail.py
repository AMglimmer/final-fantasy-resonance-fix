"""Transactional local deployment for the inspected Steam retail build only."""
import argparse, csv, hashlib, io, json, os, shutil, subprocess, sys
from datetime import datetime
from pathlib import Path

GAME = Path(r'D:\SteamLibrary\steamapps\common\FINAL FANTASY RESONANCE\FFRS\Binaries\Win64')
EXE_HASH = 'b8c588e46ebadbe37feca7bdf2da0ae63a8538099d14a53ec0c2241b086e5ffa'
FILES = ['winmm.dll', 'scripts/final_fantasy_resonance_fix.asi', 'scripts/final_fantasy_resonance_fix.toml']

def digest(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()

def ensure_stopped():
    result = subprocess.run(['tasklist.exe', '/FO', 'CSV', '/NH'], capture_output=True, check=True)
    for row in csv.reader(io.StringIO(result.stdout.decode(errors='replace'))):
        if row and row[0].lower() in ('ffrs.exe', 'ffrs-win64-shipping.exe'):
            raise RuntimeError('Exit FINAL FANTASY RESONANCE before changing patch files. DOTA2 can remain open.')

def atomic_copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    temp = target.with_name(target.name + '.ffrs-install-tmp')
    shutil.copy2(source, temp)
    os.replace(temp, target)

def restore(state, folder):
    for rel, entry in state['files'].items():
        target = GAME/rel
        disabled = target.with_suffix(target.suffix+'.disabled')
        if entry['existed']:
            atomic_copy(folder/'backup'/rel, target)
        elif target.exists():
            target.unlink()
        if disabled.exists() and digest(disabled)==entry['installed_sha256']:
            disabled.unlink()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['install', 'disable', 'enable', 'uninstall', 'status'])
    parser.add_argument('--state-dir', type=Path, default=Path(__file__).resolve().parent)
    args = parser.parse_args()
    folder = args.state_dir.resolve()
    manifest = folder/'install-manifest.json'
    if args.action=='status':
        print(manifest.read_text(encoding='utf8') if manifest.exists() else 'Not installed')
        return
    ensure_stopped()
    # This tool has no alternate game-path option: the demo can never be targeted.
    if GAME.resolve()!=GAME or 'DEMO' in str(GAME).upper(): raise RuntimeError('Unexpected target path')
    if args.action=='install':
        if manifest.exists(): raise RuntimeError('Deployment manifest already exists; inspect status first')
        if digest(GAME/'FFRS-Win64-Shipping.exe')!=EXE_HASH: raise RuntimeError('Retail EXE has changed; revalidate first')
        package=folder/'retail-candidate'
        expected=json.loads((package/'package-hashes.json').read_text())
        for rel in FILES:
            if digest(package/rel)!=expected[rel]: raise RuntimeError(f'Package checksum mismatch: {rel}')
        conflicts=[p for p in GAME.rglob('*.asi')]
        conflicts += [GAME/n for n in ['winmm.dll','dinput8.dll','dsound.dll','version.dll','dxgi.dll','d3d9.dll','xinput1_3.dll'] if (GAME/n).exists()]
        if conflicts: raise RuntimeError('Existing loader/mod requires manual integration: '+str(conflicts))
        state={'target':str(GAME),'created':datetime.now().astimezone().isoformat(),'status':'preparing','exe_sha256':EXE_HASH,'files':{}}
        for rel in FILES:
            target=GAME/rel
            entry={'existed':target.exists(),'installed_sha256':expected[rel]}
            if target.exists():
                entry['original_sha256']=digest(target)
                atomic_copy(target,folder/'backup'/rel)
            state['files'][rel]=entry
        manifest.write_text(json.dumps(state,indent=2),encoding='utf8')
        try:
            for rel in FILES:
                atomic_copy(package/rel,GAME/rel)
                if digest(GAME/rel)!=expected[rel]: raise RuntimeError('Installed checksum mismatch: '+rel)
            state['status']='installed-unvalidated'
        except BaseException:
            restore(state,folder)
            state['status']='rolled-back'
            manifest.write_text(json.dumps(state,indent=2),encoding='utf8')
            raise
    else:
        state=json.loads(manifest.read_text(encoding='utf8'))
        if state['target']!=str(GAME):raise RuntimeError('Manifest targets another directory')
        asi=GAME/FILES[1]; disabled=asi.with_suffix('.asi.disabled')
        if args.action in ('disable','enable'):
            source,target=(asi,disabled) if args.action=='disable' else (disabled,asi)
            if target.exists() and not source.exists():
                print('Already',args.action+'d'); return
            if target.exists():raise RuntimeError('Both enabled and disabled ASI files exist')
            if digest(source)!=state['files'][FILES[1]]['installed_sha256']:raise RuntimeError('ASI changed; refusing to rename')
            source.rename(target)
            state['status']='disabled' if args.action=='disable' else 'installed-unvalidated'
        elif args.action=='uninstall':
            # Preflight all files before restoring any. Preserve user config edits.
            for rel,entry in state['files'].items():
                path=GAME/rel
                if rel.endswith('.asi') and not path.exists():path=path.with_suffix('.asi.disabled')
                if path.exists() and digest(path)!=entry['installed_sha256']:
                    if rel.endswith('.toml'):atomic_copy(path,folder/'backup/user-edited-config.toml')
                    else:raise RuntimeError('File changed; refusing to remove: '+str(path))
            restore(state,folder)
            state['status']='uninstalled'
    state['updated']=datetime.now().astimezone().isoformat()
    manifest.write_text(json.dumps(state,indent=2),encoding='utf8')
    print(state['status'],GAME)

if __name__=='__main__':
    try: main()
    except Exception as error:
        print('ERROR:',error,file=sys.stderr)
        raise SystemExit(1)
