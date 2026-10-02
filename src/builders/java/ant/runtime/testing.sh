#!/bin/sh
set -eu
adapter=$(mktemp -d)
trap 'rm -rf "$adapter"' EXIT
classpath='/opt/ant/lib/*:/opt/jacoco/jacocoant.jar'
javac -cp "$classpath" -d "$adapter" /oyzu/AntTesting.java /oyzu/AntCoverage.java /oyzu/AntJUnit.java /oyzu/AntReports.java
java -cp "$adapter:$classpath" AntTesting "$1" "$2" /opt/jacoco/jacocoagent.jar "$3" "$4"
