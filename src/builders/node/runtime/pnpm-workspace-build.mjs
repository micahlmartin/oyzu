// pnpm owns workspace protocol conversion, lock importers and native packing.
import {spawnSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {readFileSync,writeFileSync} from 'node:fs';
import {dirname,join,resolve} from 'node:path';
import {entrypoint} from './native-workspace.mjs';
import {begin,projectDependencies,read,root} from './workspace-plan.mjs';
import {runWorkspace} from './workspace-build.mjs';

const native=[...entrypoint('pnpm','bin/pnpm.cjs'),'--config.manage-package-manager-versions=false'];
const require=createRequire(import.meta.url);
runWorkspace({
  project(encoded) {
    const spec=begin(encoded);
    const yaml=require(resolve(dirname(native[1]),'../../yaml'));
    const lockPath=join(root,'pnpm-lock.yaml');
    const lock=yaml.parse(readFileSync(lockPath,'utf8'));
    const versions=new Map(spec.modules.map(m=>[m.name,m.version]));
    for (const entry of [{path:'.',version:spec.rootVersion,dependencies:spec.rootDependencies},...spec.modules]) {
      const path=join(root,entry.path,'package.json');
      const pkg=read(path);
      if(entry.name && pkg.name!==entry.name)throw new Error('Workspace package identity changed');
      projectDependencies(pkg,entry.dependencies,versions);
      for(const edge of entry.dependencies) {
        for(const field of ['dependencies','devDependencies','optionalDependencies','peerDependencies']) {
          if(!Object.hasOwn(pkg[field]??{},edge.name))continue;
          const version=versions.get(edge.target);
          pkg[field][edge.name]=`workspace:${edge.name===edge.target?'':edge.target+'@'}${version}`;
          const record=lock.importers[entry.path]?.[field]?.[edge.name];
          if(record)record.specifier=pkg[field][edge.name];
        }
      }
      pkg.version=entry.version;
      writeFileSync(path,JSON.stringify(pkg,null,2)+'\n');
    }
    writeFileSync(lockPath,yaml.stringify(lock));
  },
  script:(name,member)=>[...native,'--dir',resolve(root,member?.path??'.'),'run',name],
  pack(member,directory) {
    const result=spawnSync(native[0],[...native.slice(1),'--dir',resolve(root,member.path),'--config.ignore-scripts=true','pack','--out',join(directory,member.filename)],{cwd:root,stdio:'inherit'});
    if(result.error)throw result.error;
    if(result.status!==0)throw new Error('Native pnpm packaging failed');
  },
});
