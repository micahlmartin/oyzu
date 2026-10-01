// Convert native Jest JSON results into JUnit without inventing passing cases.
function xml(value) {
  return String(value).replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\uFFFE\uFFFF]/g, '\uFFFD')
    .replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;').replaceAll("'", '&apos;');
}

export function junit(results) {
  if (!results || !Array.isArray(results.testResults) || typeof results.success !== 'boolean') {
    throw new Error('invalid Jest result document');
  }
  let total = 0, failed = 0, skipped = 0, errors = 0;
  const suites = results.testResults.map(suite => {
    if (!Array.isArray(suite.assertionResults) || typeof suite.name !== 'string') {
      throw new Error('invalid Jest suite result');
    }
    let suiteFailed = 0, suiteSkipped = 0, suiteErrors = 0;
    const cases = suite.assertionResults.map(result => {
      if (typeof result.fullName !== 'string' || !Array.isArray(result.failureMessages)) {
        throw new Error('invalid Jest assertion result');
      }
      let outcome = '';
      switch (result.status) {
        case 'passed': break;
        case 'failed':
          suiteFailed++;
          outcome = `<failure>${xml(result.failureMessages.join('\n'))}</failure>`;
          break;
        case 'pending': case 'todo': case 'disabled':
          suiteSkipped++;
          outcome = `<skipped message="${xml(result.status)}"/>`;
          break;
        default: throw new Error(`unsupported Jest assertion status ${result.status}`);
      }
      const duration = result.duration == null ? 0 : result.duration;
      if (!Number.isFinite(duration) || duration < 0) throw new Error('invalid Jest assertion duration');
      return `<testcase classname="${xml(suite.name)}" name="${xml(result.fullName)}" time="${duration / 1000}">${outcome}</testcase>`;
    });
    // Loading errors and suite-level failures can occur before any assertion.
    if (suite.status === 'failed' && suiteFailed === 0) {
      suiteErrors++;
      cases.push(`<testcase classname="${xml(suite.name)}" name="suite execution"><error>${xml(suite.message || 'Jest suite failed before reporting a failed assertion')}</error></testcase>`);
    } else if (!['passed', 'failed', 'focused', 'skipped'].includes(suite.status)) {
      throw new Error(`unsupported Jest suite status ${suite.status}`);
    }
    total += cases.length; failed += suiteFailed; skipped += suiteSkipped; errors += suiteErrors;
    return `<testsuite name="${xml(suite.name)}" tests="${cases.length}" failures="${suiteFailed}" errors="${suiteErrors}" skipped="${suiteSkipped}">${cases.join('')}</testsuite>`;
  });
  return `<?xml version="1.0" encoding="UTF-8"?>\n<testsuites tests="${total}" failures="${failed}" errors="${errors}" skipped="${skipped}">${suites.join('')}</testsuites>\n`;
}
