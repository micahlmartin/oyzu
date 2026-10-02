"""Run the shared native Ant reporting integration on captured or host inputs."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main():
    if len(sys.argv) != 6:
        raise ValueError('Ant reporting requires one native target; extra task arguments are not supported')
    mode, junit, coverage, version, target = sys.argv[1:]
    if mode == 'captured':
        ant, jacoco = Path('/opt/ant'), Path('/opt/jacoco')
    elif mode == 'host':
        if not os.environ.get('ANT_HOME') or not os.environ.get('JACOCO_HOME'):
            raise ValueError('Provision Ant and JaCoCo and set ANT_HOME and JACOCO_HOME before direct tests')
        ant = Path(os.environ['ANT_HOME']).resolve()
        jacoco = Path(os.environ['JACOCO_HOME']).resolve()
    else:
        raise ValueError('Unknown Ant test execution mode')
    for path in [ant/'lib/ant.jar', jacoco/'jacocoant.jar', jacoco/'jacocoagent.jar']:
        if not path.is_file():
            raise ValueError('Missing native Ant reporting prerequisite: ' + str(path))
    java_home = os.environ.get('JAVA_HOME')
    def executable(name):
        return str(Path(java_home)/'bin'/(name + ('.exe' if os.name == 'nt' else ''))) if java_home else name
    runtime = Path(__file__).parent
    classpath = os.pathsep.join([str(ant/'lib/*'), str(jacoco/'jacocoant.jar')])
    with tempfile.TemporaryDirectory(prefix='oyzu-ant-adapter-') as temporary:
        sources = [str(runtime/name) for name in ['AntTesting.java', 'AntCoverage.java', 'AntJUnit.java', 'AntReports.java']]
        compiled = subprocess.run([executable('javac'), '-cp', classpath, '-d', temporary, *sources])
        if compiled.returncode:
            return compiled.returncode
        return subprocess.run([executable('java'), '-cp', temporary+os.pathsep+classpath,
                               'AntTesting', junit, coverage, str(jacoco/'jacocoagent.jar'),
                               version, target]).returncode


if __name__ == '__main__':
    raise SystemExit(main())
