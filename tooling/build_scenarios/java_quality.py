"""Shared evidence obligations for native Java quality gates in captured builds."""


def verify(project, source, invoke, validate, source_files):
    original = source.read_bytes()
    for operation in ['lint', 'format-check']:
        text = original.decode('utf-8')
        if operation == 'lint':
            # All fixtures have a package declaration. This stays valid Java and
            # fails native UnusedImports rather than compilation or native tests.
            position = text.index(';') + 1
            text = text[:position] + '\nimport java.util.Random;\n' + text[position:]
        else:
            text = text.replace('public class ', 'public  class ', 1).replace('public final class ', 'public  final class ', 1)
            assert text != original.decode('utf-8'), source
        source.write_text(text, newline='\n')
        before = source_files(project)
        invoke(project, 'build', success=False)
        manifest = validate(project/'dist')
        assert source_files(project) == before
        assert not manifest['artifacts']
        assert next(a for a in manifest['actions'] if a['id']=='project:'+operation)['status']=='failed'
        assert next(a for a in manifest['actions'] if a['id']=='project:package')['status']=='blocked'
        assert any(r['kind']=='test' and r['summary']['passed']>0 for r in manifest['reports'])
        source.write_bytes(original)
