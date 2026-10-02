// Yarn's own workspace dependency classification is the source of local edges.
import {existsSync, readFileSync, realpathSync} from 'node:fs';
import {join} from 'node:path';
import {member} from './native-workspace.mjs';

export function inventory(output) {
  const logs = output.trim().split(/\r?\n/).map(line=>JSON.parse(line)).filter(e=>e.type==='log');
  if (logs.length!==1) throw new Error('Expected one native Yarn workspace inventory');
  return JSON.parse(logs[0].data);
}

export function model(output, workspace) {
  const native = inventory(output);
  const fields = [['optionalDependencies','optional'],['dependencies','prod'],['devDependencies','dev'],['peerDependencies','peer']];
  const manifest = path=>JSON.parse(readFileSync(join(workspace,path,'package.json'),'utf8'));
  function edges(pkg, names) {
    return names.map(name=>{
      const field = fields.find(([field])=>Object.hasOwn(pkg[field]??{},name));
      if (!field) throw new Error('Native Yarn workspace edge lacks its declaration');
      return {name,target:name,kind:field[1],spec:pkg[field[0]][name]};
    }).sort((a,b)=>a.name.localeCompare(b.name,'en'));
  }
  const members = Object.entries(native).map(([name,details])=>{
    const scope = member(name,details.location,workspace);
    const pkg = manifest(scope.path);
    for (const file of ['.yarnrc','.yarnrc.yml','.npmrc']) {
      if(existsSync(join(workspace,scope.path,file))) throw new Error('Captured Yarn members cannot override registry configuration');
    }
    return {...scope, version:pkg.version, private:pkg.private===true, scripts:pkg.scripts??{},
      dependencies:edges(pkg,details.workspaceDependencies)};
  }).sort((a,b)=>a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const root = manifest('.');
  const names = members.filter(m=>fields.some(([field])=>Object.hasOwn(root[field]??{},m.name))
    && existsSync(join(workspace,'node_modules',m.name))
    && realpathSync(join(workspace,'node_modules',m.name))===realpathSync(join(workspace,m.path))).map(m=>m.name);
  return {schemaVersion:1,members,rootDependencies:edges(root,names)};
}
