// Compose the native selected reporter with native XUnit; do not implement XML
// or test counting here. This adapter is qualified against Mocha 11.
const {createRequire} = require('node:module');
const {join, resolve} = require('node:path');
const project = createRequire(join(process.cwd(), 'package.json'));
const Mocha = project('mocha');

module.exports = class OyzuMochaReporter {
  constructor(runner, options) {
    // Only bare native scripts are instrumented. Mocha already loaded this
    // configuration in this process; CJS evaluation uses its native cache.
    const native = project('mocha/lib/cli/options').loadOptions([]);
    if (native.parallel) throw new Error('Mocha parallel reporting needs a qualified adapter');
    const selected = native.reporter ?? 'spec';
    let Reporter = Mocha.reporters[selected];
    if (!Reporter) {
      try { Reporter = project(selected); }
      catch (error) {
        if (error.code !== 'MODULE_NOT_FOUND') throw error;
        Reporter = project(resolve(selected));
      }
    }
    if (Reporter === module.exports) throw new Error('Recursive Oyzu Mocha reporter');
    this.native = new Reporter(runner, options);
    this.junit = new Mocha.reporters.XUnit(runner, {...options, reporterOptions:{output:process.env.OYZU_MOCHA_JUNIT}});
  }
  done(failures, callback) {
    const finish = count => this.junit.done(Math.max(failures, count), callback);
    if (typeof this.native.done === 'function') this.native.done(failures, finish);
    else finish(failures);
  }
};
