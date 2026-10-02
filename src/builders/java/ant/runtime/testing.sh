#!/bin/sh
set -eu
adapter=$(mktemp -d)
trap 'rm -rf "$adapter"' EXIT
classpath='/opt/ant/lib/ant.jar:/opt/ant/lib/ant-launcher.jar:/opt/jacoco/jacocoant.jar'
javac -cp "$classpath" -d "$adapter" /oyzu/AntTesting.java /oyzu/AntCoverage.java
java -cp "$adapter:$classpath" AntTesting "$1" "$2" /opt/jacoco/jacocoagent.jar "$3" "$4"
