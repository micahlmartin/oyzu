"""Run native pytest/coverage against installed sources shadowed by a checkout."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
REPORTER = ROOT / 'src/builders/python/runtime/reporting.py'


class InstalledCoverageTests(unittest.TestCase):
    def run_case(self, flat=False, threshold=False):
        with tempfile.TemporaryDirectory() as temporary:
            root=Path(temporary)
            site=root/'installed'
            checkout=root/'checkout'
            site.mkdir()
            checkout.mkdir()
            module='oyzu_cov_probe'
            relative=module+'.py' if flat else module+'/__init__.py'
            installed=site/relative
            installed.parent.mkdir(parents=True,exist_ok=True)
            installed.write_text('def greeting(name):\n    if name:\n        return "Hi " + name\n    return "Hi"\n')
            shadow=checkout/relative
            shadow.parent.mkdir(parents=True,exist_ok=True)
            shadow.write_text('raise AssertionError("checkout imported instead of installed artifact")\n')
            metadata=site/'oyzu_cov_probe-1.0.dist-info'
            metadata.mkdir()
            (metadata/'METADATA').write_text('Metadata-Version: 2.1\nName: oyzu-cov-probe\nVersion: 1.0\n')
            (metadata/'RECORD').write_text(relative+',,\noyzu_cov_probe-1.0.dist-info/METADATA,,\n')
            (checkout/'tests').mkdir()
            script='from oyzu_cov_probe import greeting\ndef test_greeting():\n    assert greeting("Ada") == "Hi Ada"\n'
            if not threshold:
                script+='    assert greeting("") == "Hi"\n'
            (checkout/'tests/test_greeting.py').write_text(script)
            if flat:
                (checkout/'.coveragerc').write_text('[run]\nbranch=true\nsource=oyzu_cov_probe\n[report]\nfail_under=100\n')
            else:
                (checkout/'pyproject.toml').write_text('[tool.coverage.run]\nbranch=true\n[tool.coverage.report]\nfail_under=100\n')
            code='''import importlib.util,sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
spec=importlib.util.spec_from_file_location('reporter',sys.argv[2])
reporter=importlib.util.module_from_spec(spec); spec.loader.exec_module(reporter)
raise SystemExit(reporter.run_tests('oyzu-cov-probe',Path('junit.xml'),Path('coverage.xml')))
'''
            result=subprocess.run([sys.executable,'-I','-c',code,str(site),str(REPORTER)],cwd=checkout,capture_output=True,text=True,timeout=60)
            self.assertEqual(result.returncode,1 if threshold else 0,result.stdout+'\n'+result.stderr)
            coverage=ET.parse(checkout/'coverage.xml').getroot()
            self.assertEqual(int(coverage.attrib['lines-valid']),4)
            self.assertGreater(int(coverage.attrib['lines-covered']),0)
            self.assertGreater(int(coverage.attrib['branches-valid']),0)
            classes=coverage.findall('.//class')
            self.assertEqual(len(classes),1)
            self.assertNotIn('test_greeting',classes[0].attrib['filename'])
            self.assertEqual(shadow.read_text(),'raise AssertionError("checkout imported instead of installed artifact")\n')

    def test_installed_package_is_measured_instead_of_checkout_or_tests(self):
        self.run_case()

    def test_installed_single_file_module_is_measured(self):
        self.run_case(flat=True)

    def test_native_coverage_threshold_fails_even_when_tests_pass(self):
        self.run_case(threshold=True)


if __name__=='__main__':
    unittest.main()
