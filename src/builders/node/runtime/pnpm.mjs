import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {run} from './manager-runtime.mjs';

export const profile = {
  id:'pnpm', lock:'pnpm-lock.yaml',
  command:['pnpm','--config.manage-package-manager-versions=false'],
  install:(mode, temporary) => ['install','--offline','--frozen-lockfile',
    `--ignore-scripts=${mode === 'acquire'}`,'--config.engine-strict=true','--config.force=false',
    '--store-dir',join(temporary,'store')],
};
if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) await run(profile);
