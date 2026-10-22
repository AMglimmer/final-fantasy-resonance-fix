"""Read-only preflight of the two inspected Steam executables. No third-party modules."""
import argparse, hashlib, json, re, struct
from pathlib import Path

PROFILES = {
    'e06401cdf4e64d55f652043bdd2012267d5529a00c3dbc36ee4365e0ce49caad': 'demo build 24032336',
    'b8c588e46ebadbe37feca7bdf2da0ae63a8538099d14a53ec0c2241b086e5ffa': 'retail build 24250053',
}

def inspect(path):
    data = path.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    nt = struct.unpack_from('<I', data, 0x3c)[0]
    machine, section_count = struct.unpack_from('<HH', data, nt+4)
    optional_size = struct.unpack_from('<H', data, nt+20)[0]
    sections = []
    for i in range(section_count):
        at = nt+24+optional_size+40*i
        virtual_size, rva, raw_size, raw = struct.unpack_from('<IIII', data, at+8)
        sections.append((rva, raw, raw_size))
    def to_rva(offset):
        for rva, raw, size in sections:
            if raw <= offset < raw+size: return rva+offset-raw
        raise ValueError('Signature outside PE sections')
    checks = []
    root = Path(__file__).resolve().parents[1]
    for source in [root/'src/runtime.rs', *(root/'src/fixes').glob('*.rs')]:
        prefix = '' if source.name=='runtime.rs' else r'(?:signature:|const\s+\w+:\s*&str\s*=)\s*'
        for match in re.finditer(prefix+r'"((?:[0-9A-F?]{2} ){7,}[0-9A-F?]{2})"', source.read_text()):
            pattern = match.group(1)
            # Patch byte strings and test inputs are shorter than this limit.
            rx = b''.join(b'.' if b=='??' else re.escape(bytes.fromhex(b)) for b in pattern.split())
            offsets = [m.start() for m in re.finditer(rx, data, re.S)]
            checks.append({'source':str(source.relative_to(root)), 'pattern':pattern,
                           'rvas':[hex(to_rva(at)) for at in offsets], 'unique':len(offsets)==1})
    result = {'path':str(path),'sha256':digest,'profile':PROFILES.get(digest),
              'machine':hex(machine),'signatures':checks}
    result['passed'] = bool(result['profile']) and machine==0x8664 and all(c['unique'] for c in checks)
    return result

if __name__=='__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    result = inspect(args.executable)
    if args.output: args.output.write_text(json.dumps(result, indent=2), encoding='utf8')
    print(result['profile'], 'PASS' if result['passed'] else 'FAIL',
          f"({len(result['signatures'])} signatures)")
    for item in result['signatures']:
        if not item['unique']: print('NOT UNIQUE:', item['source'], item['pattern'], 'count', len(item['rvas']))
    raise SystemExit(0 if result['passed'] else 1)
