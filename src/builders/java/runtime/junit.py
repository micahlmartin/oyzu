"""Compose native suites without manufacturing outcomes or interpreting counts."""
import xml.etree.ElementTree as ET


def combine(contents, destination):
    suites = ET.Element('testsuites')
    found = False
    for data in contents:
        native = ET.fromstring(data)
        if native.tag not in {'testsuite', 'testsuites'}:
            raise ValueError('Unexpected native JUnit root')
        suites.append(native)
        found = True
    if found:
        ET.ElementTree(suites).write(destination, encoding='utf-8', xml_declaration=True)
