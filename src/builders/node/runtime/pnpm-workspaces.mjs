// Native pnpm lists membership and validates lock importers before this projection.
import {existsSync, readFileSync, realpathSync} from 'node:fs';
import {join, resolve} from 'node:path';
import {member} from './native-workspace.mjs';

export function model(native, workspace, lock) {
  if (!Array.isArray(native)) throw new Error('Expected native pnpm workspace list');
  const root = realpathSync(workspace);
  const members = native.filter(p=>realpathSync(p.path)!==root).map(p=>{
    const scope=member(p.name,p.path,workspace);
    const pkg=JSON.parse(readFileSync(join(workspace,scope.path,'package.json'),'utf8'));
    for (const file of ['.npmrc','.pnpmfile.cjs']) {
      if (existsSync(join(workspace,scope.path,file))) throw new Error('Captured pnpm members cannot override registry configuration');
    }
    return {...scope,version:pkg.version,private:pkg.private===true,scripts:pkg.scripts??{},dependencies:[]};
  });
  const byPath=new Map(members.map(m=>[realpathSync(join(workspace,m.path)),m]));
  function edges(path) {
    const importer=lock.importers[path];
    if (!importer) throw new Error('Native pnpm member lacks a lock importer');
    const pkg=JSON.parse(readFileSync(join(workspace,path,'package.json'),'utf8'));
    const result=new Map();
    for (const [field,kind] of [['optionalDependencies','optional'],['dependencies','prod'],['devDependencies','dev']]) {
      for (const [name,entry] of Object.entries(importer[field]??{})) {
        if (!entry.version.startsWith('link:')) continue;
        const target=byPath.get(realpathSync(resolve(workspace,path,entry.version.slice(5))));
        if (!target) throw new Error('pnpm local link must resolve to a captured workspace member');
        if (pkg[field]?.[name]!==entry.specifier) throw new Error('pnpm local reference differs from its frozen lock');
        if (!result.has(name)) result.set(name,{name,target:target.name,kind,spec:entry.specifier});
      }
    }
    return [...result.values()];
  }
  const rootDependencies=edges('.');
  if (!members.length) {
    if(rootDependencies.length) throw new Error('pnpm links require captured workspace members');
    return null;
  }
  for (const m of members) m.dependencies=edges(m.path);
  members.sort((a,b)=>a.path<b.path?-1:a.path>b.path?1:0);
  return {schemaVersion:1,members,rootDependencies};
}
