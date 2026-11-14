#!/usr/bin/env python3
"""Bind all platform binaries to one immutable engine revision and protocol."""
import argparse, hashlib, json, re, subprocess
from pathlib import Path

def contract(root, out, commit):
    if not re.fullmatch('[0-9a-f]{40}', commit): raise ValueError('Full engine commit required')
    source = (root/'crates/horizon-core/src/version.rs').read_text()
    def integer(key): return int(re.search(r'const '+key+r':[^=]+=\s*(\d+)',source)[1])
    def pair(key): return list(map(int,re.search(r'const '+key+r':[^=]+=\s*\((\d+),\s*(\d+)\)',source).groups()))
    version = re.search(r'const VANILLA_VERSION:[^=]+=\s*"([^"]+)"',source)[1]
    targets = {}
    for platform, name in [('linux-x86_64','vis-horizon-linux-x86_64.tar.gz'),('windows-x64','vis-horizon-windows-x64.zip'),('android-arm64','vis-horizon-android-arm64.apk')]:
        path = out/name
        with path.open('rb') as stream: digest=hashlib.file_digest(stream,'sha256').hexdigest()
        targets[platform]={'asset':name,'sha256':digest,'size':path.stat().st_size}
    android=json.loads((out/'android-engine.json').read_text())
    if android['engine_commit'] != commit or android['package']!='com.visstudios.mcrust': raise ValueError('Android engine differs from PC')
    if not re.fullmatch('[0-9a-f]{64}',android['certificate_sha256']): raise ValueError('Signing certificate required')
    if not isinstance(android['version_code'], int) or not 1 <= android['version_code'] <= 2100000000: raise ValueError('Android version code outside platform limits')
    targets['android-arm64'].update({key:android[key] for key in ['package','version_code','certificate_sha256']})
    return dict(schema=1,source_repo='https://github.com/JoseDaniel03/horizon-VISNEXO.git',engine_commit=commit,minecraft=version,protocol=integer('PROTOCOL_VERSION'),data_version=integer('DATA_VERSION'),resource_pack=pair('RESOURCE_PACK_FORMAT'),data_pack=pair('DATA_PACK_FORMAT'),targets=targets)

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--out',type=Path,required=True);p.add_argument('--commit',required=True);a=p.parse_args()
    root=Path(__file__).resolve().parents[1]
    # CI checks out the same revision for PC and Android jobs.
    actual=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
    if actual!=a.commit: raise ValueError('Release checkout mismatch')
    data=contract(root,a.out,a.commit)
    (a.out/'vis-mcrust-release.json').write_text(json.dumps(data,indent=2)+'\n')
