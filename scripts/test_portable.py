"""Exercise portable deployment against temporary fixtures, never a real game.

Get-Process is mocked only inside each test child PowerShell process so an
unrelated running game does not block fixture tests. The shipped script keeps
its real process guard, which is also tested explicitly below.
"""
import argparse, hashlib, json, shutil, subprocess, tempfile
from pathlib import Path

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def quote(value): return "'"+str(value).replace("'","''")+"'"

def test(package, workspace):
    workspace=workspace.resolve()
    if not workspace.is_dir(): raise ValueError('Existing test workspace required')
    root=Path(tempfile.mkdtemp(prefix='portable-',dir=workspace)).resolve()
    if not root.is_relative_to(workspace): raise ValueError('Fixture escaped test workspace')
    results=[]
    def fixture(name):
        folder=root/name
        bundle=folder/'bundle'; game=folder/'game'
        shutil.copytree(package,bundle)
        game.mkdir()
        exe=game/'FFRS-Win64-Shipping.exe'; exe.write_bytes(b'TEST FIXTURE - NEVER EXECUTED')
        metadata=json.loads((bundle/'package.json').read_text())
        metadata['retailExeSha256']=sha(exe)
        (bundle/'package.json').write_text(json.dumps(metadata))
        return bundle,game
    def invoke(bundle,game,action,expected=0,running=False):
        mock="function Get-Process { param($Name,$ErrorAction) " + ("[pscustomobject]@{ProcessName='FFRS'}" if running else "") + " }; "
        command=mock+'& '+quote(bundle/'Manage-Patch.ps1')+' -Action '+action+' -GameDirectory '+quote(game)+' -NonInteractive'
        result=subprocess.run(['powershell.exe','-NoProfile','-ExecutionPolicy','Bypass','-Command',command],capture_output=True,text=True,timeout=30)
        if result.returncode!=expected:
            raise AssertionError(f'{action} returned {result.returncode}, expected {expected}: {result.stdout}\n{result.stderr}')
        return result
    bundle,game=fixture('fresh')
    invoke(bundle,game,'Install')
    assert (game/'scripts/final_fantasy_resonance_fix.asi').exists()
    invoke(bundle,game,'Disable'); assert (game/'scripts/final_fantasy_resonance_fix.asi.disabled').exists()
    invoke(bundle,game,'Enable'); assert not (game/'scripts/final_fantasy_resonance_fix.asi.disabled').exists()
    invoke(bundle,game,'Uninstall')
    assert not (game/'winmm.dll').exists() and not (game/'scripts/final_fantasy_resonance_fix.asi').exists()
    results.append('fresh install / disable / enable / uninstall')
    bundle,game=fixture('restore')
    (game/'scripts').mkdir()
    shutil.copy2(bundle/'payload/winmm.dll',game/'winmm.dll')
    old_asi=b'PREVIOUS PATCH'; old_config=b'super_enable = false\n'
    asi=game/'scripts/final_fantasy_resonance_fix.asi'; asi.write_bytes(old_asi)
    config=game/'scripts/final_fantasy_resonance_fix.toml'; config.write_bytes(old_config)
    invoke(bundle,game,'Install'); assert config.read_bytes()==old_config
    edited=b'super_enable = true\n# user edit\n'; config.write_bytes(edited)
    invoke(bundle,game,'Disable'); invoke(bundle,game,'Uninstall')
    assert asi.read_bytes()==old_asi and config.read_bytes()==old_config
    assert any(p.read_bytes()==edited for p in (bundle/'.ffrs-backups').glob('user-config-*.toml'))
    results.append('restore previous ASI/config and preserve user edits; uninstall while disabled')
    bundle,game=fixture('wrong-build')
    (game/'FFRS-Win64-Shipping.exe').write_bytes(b'UNSUPPORTED BUILD')
    invoke(bundle,game,'Install',expected=1)
    assert not (game/'winmm.dll').exists() and not (bundle/'.ffrs-portable-state.json').exists()
    results.append('unsupported build rejected without writes')
    bundle,game=fixture('foreign-loader')
    loader=game/'winmm.dll'; loader.write_bytes(b'OTHER LOADER')
    invoke(bundle,game,'Install',expected=1)
    assert loader.read_bytes()==b'OTHER LOADER' and not (bundle/'.ffrs-portable-state.json').exists()
    results.append('foreign loader preserved')
    bundle,game=fixture('changed-asi')
    invoke(bundle,game,'Install')
    (game/'scripts/final_fantasy_resonance_fix.asi').write_bytes(b'USER REPLACEMENT')
    invoke(bundle,game,'Uninstall',expected=1)
    assert (game/'winmm.dll').exists() and (game/'scripts/final_fantasy_resonance_fix.asi').read_bytes()==b'USER REPLACEMENT'
    results.append('changed ASI blocks uninstall before any removal')
    bundle,game=fixture('running-game')
    invoke(bundle,game,'Install',expected=1,running=True)
    assert not (bundle/'.ffrs-portable-state.json').exists()
    results.append('running game blocks installation')
    print(json.dumps({'passed':len(results),'checks':results,'fixtures':str(root)},indent=2))

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package',type=Path)
    parser.add_argument('workspace',type=Path)
    args=parser.parse_args(); test(args.package,args.workspace)
