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
    def run_case(self, flat=False, threshold=False, layout='conventional'):
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
            directory = checkout / ('checks' if layout == 'configured' else 'tests')
            directory.mkdir()
            script='from oyzu_cov_probe import greeting\ndef test_greeting():\n    assert greeting("Ada") == "Hi Ada"\n'
            if not threshold:
                script+='    assert greeting("") == "Hi"\n'
            test_file = directory / ('spec_greeting.py' if layout == 'configured' else 'test_greeting.py')
            if layout == 'root':
                directory.rmdir()
                test_file = checkout/'test_greeting.py'
            if layout == 'empty':
                directory.rmdir()
            else:
                test_file.write_text(script)
            if layout == 'configured':
                (checkout/'pytest.ini').write_text('[pytest]\ntestpaths=checks\npython_files=spec_*.py\n')
                (checkout/'test_poison.py').write_text('raise RuntimeError("native testpaths ignored")\n')
            if flat:
                (checkout/'.coveragerc').write_text('[run]\nbranch=true\nsource=oyzu_cov_probe\n[report]\nfail_under=100\n')
            elif layout != 'empty':
                (checkout/'pyproject.toml').write_text('[tool.coverage.run]\nbranch=true\n[tool.coverage.report]\nfail_under=100\n')
            code='''import importlib.util,sys
from pathlib import Path
sys.path.insert(0,sys.argv[1])
spec=importlib.util.spec_from_file_location('reporter',sys.argv[2])
reporter=importlib.util.module_from_spec(spec); spec.loader.exec_module(reporter)
raise SystemExit(reporter.run_tests('oyzu-cov-probe',Path('junit.xml'),Path('coverage.xml')))
'''
            result=subprocess.run([sys.executable,'-I','-c',code,str(site),str(REPORTER)],cwd=checkout,capture_output=True,text=True,timeout=60)
            expected = 5 if layout == 'empty' else 1 if threshold else 0
            self.assertEqual(result.returncode,expected,result.stdout+'\n'+result.stderr)
            junit = ET.parse(checkout/'junit.xml')
            cases = junit.findall('.//testcase')
            self.assertEqual(len(cases), 0 if layout == 'empty' else 1)
            self.assertFalse(junit.findall('.//failure'))
            if layout == 'empty':
                # Keep native no-tests evidence; do not invent successful tests
                # or require a coverage report the runner did not produce.
                if (checkout/'coverage.xml').exists():
                    coverage = ET.parse(checkout/'coverage.xml').getroot()
                    self.assertEqual(int(coverage.attrib['lines-covered']), 0)
                return
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

    def test_root_level_tests_use_native_collection(self):
        self.run_case(layout='root')

    def test_native_testpaths_and_patterns_select_nonconventional_tests(self):
        self.run_case(layout='configured')

    def test_empty_suite_preserves_native_exit_and_zero_test_report(self):
        self.run_case(layout='empty')


if __name__=='__main__':
    unittest.main()
