"""Apply the bounded VIS handoff to the reviewed, pinned Android port.

The desktop fork stays untouched. All upstream license notices remain intact.
"""
from pathlib import Path
import subprocess
import sys

PIN = 'fa548eb7abe91c194d3780e4580b9746323cd606'
PACKAGE = 'com.visstudios.mcrust.preview'

def replace_once(path, before, after):
    text = path.read_text()
    if text.count(before) != 1:
        raise ValueError(f'Pinned port contract changed: {path}')
    path.write_text(text.replace(before, after, 1))

def apply(root):
    revision = subprocess.check_output(['git', '-C', str(root), 'rev-parse', 'HEAD'], text=True).strip()
    if revision != PIN:
        raise ValueError('Refusing an unreviewed Android source revision')
    client = root / 'crates/horizon-client/src'
    (client/'android/vis_nexo.rs').write_text((Path(__file__).parent/'vis_nexo.rs').read_text())
    replace_once(client/'android_main.rs', 'mod launcher;', 'mod launcher;\n#[path = "android/vis_nexo.rs"]\nmod vis_nexo;')
    launcher = client/'android/launcher.rs'
    replace_once(launcher, '    home: PathBuf,', '    home: PathBuf,\n    nexo_pending: bool,')
    replace_once(launcher, '        let mut catalog = Catalog::new(&home);', '''        let mut catalog = Catalog::new(&home);
        let nexo = crate::vis_nexo::request();
        if let Some(request) = &nexo {
            catalog.config.name = request.name.clone();
            catalog.config.server = request.server.clone();
        }''')
    replace_once(launcher, '            home,\n', '            home,\n            nexo_pending: nexo.is_some(),\n')
    replace_once(launcher, '        self.catalog.poll(&self.home);', '''        self.catalog.poll(&self.home);
        // A user pressed Play in Nexo. Prepare the pinned game resources, then
        // enter the same native game path as this port's Play button.
        if self.nexo_pending && self.error.is_none() && self.window.is_some()
            && self.worker.is_none() && self.session_worker.is_none()
            && !self.launch_pending_tick && !self.login.pending()
        {
            self.nexo_pending = !horizon_client::official_install::ready(&self.home);
            if let Err(error) = self.action(events) {
                self.nexo_pending = false;
                self.fail(error);
            }
        }''')
    manifest=root/'packaging/android/AndroidManifest.xml'
    replace_once(manifest, 'android:label="Horizon Android"', 'android:label="VIS McRust Preview"')
    replace_once(manifest, 'android:name="dev.horizon.android.HorizonActivity" android:exported="false"', 'android:name="dev.horizon.android.HorizonActivity" android:exported="true"')
    # Package and APK audit must agree; preserve all public payload checks.
    build=root/'tools/android_build.py'
    text=build.read_text().replace('dev.horizon.android.dev',PACKAGE)
    text=text.replace("features = ['--no-default-features', '--features', 'audio'] if legacy else []", "features = ['--no-default-features', '--features', 'audio']")
    build.write_text(text)
    audit=root/'tools/check_android_release.py'
    text=audit.read_text().replace("choices=['dev.horizon.android', 'dev.horizon.android.dev']", "choices=['dev.horizon.android', 'dev.horizon.android.dev', '"+PACKAGE+"']")
    audit.write_text(text)

if __name__ == '__main__':
    apply(Path(sys.argv[1]).resolve())
