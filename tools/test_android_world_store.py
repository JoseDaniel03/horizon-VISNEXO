"""Run the real Java storage provider against filesystem fixtures, without a phone.
Android stubs model only Context/Binder/Bundle; APK CI validates actual Android APIs.
"""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
STUBS = {
 'android/content/Context.java': '''package android.content; public class Context {
 public static java.io.File home,external; public static String caller="com.visstudios.nexo.mobile";
 static final SharedPreferences prefs=new SharedPreferences();
 public java.io.File getFilesDir(){return home;} public java.io.File getExternalFilesDir(String s){return external;}
 public Object getSystemService(String s){return new ExitManager();}
 public SharedPreferences getSharedPreferences(String s,int i){return prefs;}
 public android.content.pm.PackageManager getPackageManager(){return new android.content.pm.PackageManager();}}
''',
 'android/content/ExitManager.java': '''package android.content; public class ExitManager {
 public java.util.List<ExitInfo> getHistoricalProcessExitReasons(String p,int pid,int count){return java.util.List.of(new ExitInfo());}}''',
 'android/content/ExitInfo.java': '''package android.content; public class ExitInfo {
 public int getReason(){return 3;} public int getStatus(){return 9;} public long getTimestamp(){return 12345;}}''',
 'android/content/SharedPreferences.java': '''package android.content; public class SharedPreferences {
 java.util.HashMap<String,String> data=new java.util.HashMap<>();
 public String getString(String k,String d){return data.getOrDefault(k,d);}
 public Editor edit(){return new Editor();} public class Editor {
 public Editor putString(String k,String v){data.put(k,v);return this;}
 public Editor remove(String k){data.remove(k);return this;} public boolean commit(){return true;}}}''',
 'android/content/pm/PackageManager.java': 'package android.content.pm; public class PackageManager {public String[] getPackagesForUid(int i){return new String[]{android.content.Context.caller};}}',
 'android/app/Activity.java': 'package android.app; public class Activity extends android.content.Context {}',
 'android/os/Binder.java': 'package android.os; public class Binder {public static int getCallingUid(){return 1;}}',
 'android/os/Bundle.java': '''package android.os; public class Bundle extends java.util.HashMap<String,Object> {
 public String getString(String k){return (String)get(k);} public String getString(String k,String d){return containsKey(k)?getString(k):d;}
 public void putString(String k,String v){put(k,v);} public void putBoolean(String k,boolean v){put(k,v);}
 public boolean getBoolean(String k){return Boolean.TRUE.equals(get(k));}}''',
 'android/net/Uri.java':'package android.net; public class Uri {}',
 'android/content/ContentValues.java':'package android.content; public class ContentValues {}',
 'android/database/Cursor.java':'package android.database; public interface Cursor {}',
 'android/content/ContentProvider.java': '''package android.content; import android.net.Uri; import android.os.Bundle; import android.database.Cursor;
 public abstract class ContentProvider {public Context getContext(){return new Context();}
 public abstract Bundle call(String m,String a,Bundle b); public abstract boolean onCreate();
 public abstract Cursor query(Uri u,String[] p,String s,String[] a,String o);public abstract String getType(Uri u);
 public abstract Uri insert(Uri u,ContentValues v);public abstract int delete(Uri u,String s,String[] a);public abstract int update(Uri u,ContentValues v,String s,String[] a);}''',
}
HARNESS = '''import android.content.Context;import android.os.Bundle;import dev.horizon.android.VisGameStore;
import java.io.File;import java.nio.file.Files;
public class StorageTest {
 static final String ID="a".repeat(32);static VisGameStore store=new VisGameStore();
 static Bundle request(String id,String storage){Bundle b=new Bundle();b.putString("owner","jose");b.putString("instance",id);b.putString("storage",storage);return b;}
 static void check(boolean condition){if(!condition)throw new AssertionError();}
 public static void main(String[] args)throws Exception {
  Context.home=new File(args[0],"internal");Context.external=new File(args[0],"external");
  Context.home.mkdirs();Context.external.mkdirs();Bundle b=request(ID,"external");
  check(store.call("prepare",null,b).getBoolean("ok"));
  File world=new File(VisGameStore.begin(new android.app.Activity(),"jose",ID));
  check(world.getPath().startsWith(Context.external.getPath()));
  File saved=new File(world,"level.dat");Files.writeString(saved.toPath(),"world");
  check(!store.call("delete",null,b).getBoolean("ok"));check(saved.exists());VisGameStore.end();
  check(!store.call("prepare",null,request(ID,"internal")).getBoolean("ok"));check(saved.exists());
  Context.caller="malicious.app";try{store.call("delete",null,b);throw new AssertionError();}catch(SecurityException expected){}
  try{store.call("diagnostics",null,null);throw new AssertionError();}catch(SecurityException expected){}
  Context.caller="com.visstudios.nexo.mobile";
  check(store.call("diagnostics",null,null).getString("report").contains("No engine log"));
  File log=new File(Context.home,"logs/latest.log");log.getParentFile().mkdirs();
  Files.writeString(log.toPath(),"x".repeat(10000)+"\\n[00:00:00] [main/ERROR]: GPU failed token=supersecret /data/user/0/private/path\\n");
  String report=store.call("diagnostics",null,null).getString("report");
  check(report.contains("Android exit reason: 3; status: 9; time: 12345"));
  check(report.contains("GPU failed") && !report.contains("supersecret") && !report.contains("/data/user"));
  check(report.length()<=2000);
  Files.delete(log.toPath());Files.createSymbolicLink(log.toPath(),new File(Context.home,"logs/other.log").toPath());
  check(store.call("diagnostics",null,null).getString("report").contains("No engine log"));
  check(!store.call("delete",null,request("../escape","internal")).getBoolean("ok"));
  File outside=new File(args[0],"outside");outside.mkdirs();File precious=new File(outside,"keep");Files.writeString(precious.toPath(),"safe");
  Files.createSymbolicLink(new File(world,"link").toPath(),outside.toPath());
  check(store.call("delete",null,b).getBoolean("ok"));check(!world.exists());check(precious.exists());
  check(store.call("delete",null,b).getBoolean("ok")); // idempotent retry
  File owner=new File(Context.home,"vis-worlds/jose");owner.mkdirs();
  Files.createSymbolicLink(new File(owner,ID).toPath(),outside.toPath());
  check(!store.call("delete",null,b).getBoolean("ok"));check(precious.exists());
  System.out.println("Storage provider: owner validation, active-game exclusion, location, deletion, retries and symlinks passed");
 }
}'''

class WorldStoreTests(unittest.TestCase):
    @unittest.skipUnless(shutil.which('javac') and shutil.which('java'), 'JDK required')
    def test_real_provider(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            for name,source in STUBS.items():
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_text(source)
            provider=ROOT/'packaging/android/java/dev/horizon/android/VisGameStore.java'
            (root/'StorageTest.java').write_text(HARNESS)
            sources=[str(p) for p in root.rglob('*.java')]+[str(provider)]
            subprocess.run(['javac','-d',str(root/'classes'),*sources],check=True,capture_output=True)
            result=subprocess.run(['java','-cp',str(root/'classes'),'StorageTest',str(root/'data')],check=True,capture_output=True,text=True)
            self.assertIn('passed',result.stdout)

if __name__=='__main__':unittest.main()
