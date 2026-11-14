import json,tempfile,unittest
from pathlib import Path
from vis_mcrust_release import contract
ROOT=Path(__file__).resolve().parents[1]
class SharedRelease(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.out=Path(self.tmp.name)
  for name in ['vis-horizon-linux-x86_64.tar.gz','vis-horizon-windows-x64.zip','vis-horizon-android-arm64.apk']:(self.out/name).write_bytes(b'disposable fixture')
  self.android=dict(engine_commit='a'*40,package='com.visstudios.mcrust',version_code=1001,certificate_sha256='b'*64)
 def generate(self):
  (self.out/'android-engine.json').write_text(json.dumps(self.android));return contract(ROOT,self.out,'a'*40)
 def test_revision_and_protocol_for_all_platforms(self):
  result=self.generate();self.assertEqual(len(result['targets']),3);self.assertEqual(result['engine_commit'],'a'*40);self.assertEqual(result['protocol'],777);self.assertEqual(result['data_version'],5023)
 def test_separate_engine_revision_refuses_publication(self):
  self.android['engine_commit']='c'*40
  with self.assertRaises(ValueError):self.generate()
 def test_preview_package_refuses_publication(self):
  self.android['package']+='.preview'
  with self.assertRaises(ValueError):self.generate()
 def test_missing_platform_refuses_publication(self):
  (self.out/'vis-horizon-windows-x64.zip').unlink()
  with self.assertRaises(FileNotFoundError):self.generate()
 def test_invalid_signer_refuses_publication(self):
  self.android['certificate_sha256']=''
  with self.assertRaises(ValueError):self.generate()
if __name__=='__main__':unittest.main()
