// Yarn owns native script dispatch, private manifest projection and packing.
import {spawnSync} from 'node:child_process';
import {writeFileSync} from 'node:fs';
import {dirname,join,resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {entrypoint} from './native-workspace.mjs';
import {begin,projectDependencies,read,root} from './workspace-plan.mjs';
import {runWorkspace} from './workspace-build.mjs';

const runtime = dirname(fileURLToPath(import.meta.url));
const native = [...entrypoint('yarn','bin/yarn.js'),'--offline','--non-interactive','--ignore-path'];
function execute(command) {
  const result=spawnSync(command[0],command.slice(1),{cwd:root,stdio:'inherit'});
  if(result.error)throw result.error;
  if(result.status!==0)throw new Error('Native Yarn packaging failed');
}
runWorkspace({
  project(encoded) {
    const spec=begin(encoded);
    const versions=new Map(spec.modules.map(m=>[m.name,m.version]));
    for(const entry of [{path:'.',version:spec.rootVersion,dependencies:spec.rootDependencies},...spec.modules]) {
      const path=join(root,entry.path,'package.json');
      const pkg=read(path);
      if(entry.name && pkg.name!==entry.name)throw new Error('Workspace package identity changed');
      projectDependencies(pkg,entry.dependencies,versions);
      pkg.version=entry.version;
      writeFileSync(path,JSON.stringify(pkg,null,2)+'\n');
    }
  },
  script:(name,member)=>[...native,'--cwd',resolve(root,member?.path??'.'),'run',name],
  pack(member,directory) {
    const file=join(directory,member.filename);
    execute([...native,'--cwd',resolve(root,member.path),'--ignore-scripts','pack','--filename',file]);
    execute([process.execPath,join(runtime,'node-archive.mjs'),file,resolve(dirname(native[1]),'../../tar-stream')]);
  },
});
