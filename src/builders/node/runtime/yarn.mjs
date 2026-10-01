import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
import {run} from './manager-runtime.mjs';

export const profile = {
  id:'yarn', lock:'yarn.lock', command:['yarn'],
  install:(mode, temporary) => ['install','--offline','--frozen-lockfile','--non-interactive',
    ...(mode === 'acquire' ? ['--ignore-scripts'] : []),
    '--cache-folder',join(temporary,'cache')],
};
if (process.argv[1] && pathToFileURL(process.argv[1]).href === import.meta.url) await run(profile);
