// npm retains native script, root staging and pack receipt semantics.
import {mkdtempSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {npm,npmCommand} from './npm-native.mjs';
import {project} from './npm-workspace-plan.mjs';
import {root} from './workspace-plan.mjs';
import {packRoot} from './npm-workspace-root.mjs';
import {runWorkspace} from './workspace-build.mjs';
const cache = mkdtempSync(join(tmpdir(),'oyzu-npm-workspace-'));
try {
  runWorkspace({project,
    script:(name,member)=>npmCommand(['run',name,...(member?['--workspace',member.name]:[]),'--'],cache),
    pack:(member,directory)=>{
      const packed = member.path==='.' ? packRoot(root,directory,cache)
        : JSON.parse(npm(['pack','--ignore-scripts','--json','--workspace',member.name,'--pack-destination',directory],root,cache));
      if(packed.length!==1 || packed[0].filename!==member.filename || packed[0].name!==member.name || packed[0].version!==member.version) throw new Error('Native npm pack differs from planned artifact');
    },
  });
} finally { rmSync(cache,{recursive:true,force:true}); }
