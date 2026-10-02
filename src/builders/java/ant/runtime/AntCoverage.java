import java.nio.file.*;
import java.util.Set;
import org.jacoco.core.analysis.*;
import org.jacoco.core.tools.ExecFileLoader;
import org.jacoco.report.DirectorySourceFileLocator;
import org.jacoco.report.xml.XMLFormatter;

/** Coverage over application classes owned by the conventional src tree. */
class AntCoverage {
    static void write(Path root, Path data, Set<Path> directories, Path output) throws Exception {
        if (directories.isEmpty()) throw new IllegalArgumentException("No conventional Ant application javac output was observed");
        if (Files.size(data) > 64 * 1024 * 1024) throw new IllegalArgumentException("Ant execution data exceeds limit");
        var loader = new ExecFileLoader();
        loader.load(data.toFile());
        var coverage = new CoverageBuilder();
        var analyzer = new Analyzer(loader.getExecutionDataStore(), clazz -> {
            String source = clazz.getSourceFileName();
            if (source == null) {
                // javac -g:none omits SourceFile. Only accept conventional names.
                String name = clazz.getName().substring(clazz.getName().lastIndexOf('/') + 1);
                source = name.split("\\$", 2)[0] + ".java";
            }
            Path owner = root.resolve("src").resolve(clazz.getPackageName()).resolve(source).normalize();
            if (owner.startsWith(root.resolve("src")) && Files.isRegularFile(owner, LinkOption.NOFOLLOW_LINKS))
                coverage.visitCoverage(clazz);
            else {
                Path test = root.resolve("test").resolve(clazz.getPackageName()).resolve(source).normalize();
                if (!test.startsWith(root.resolve("test")) || !Files.isRegularFile(test, LinkOption.NOFOLLOW_LINKS))
                    throw new IllegalArgumentException("Cannot determine source ownership for Ant class " + clazz.getName() + "; preserve SourceFile debug metadata or configure explicit reports");
            }
        });
        long total = 0;
        for (Path directory : directories) {
            if (!directory.toRealPath().startsWith(root)) throw new IllegalArgumentException("Ant classes escape project");
            try (var paths = Files.walk(directory)) {
                for (Path path : paths.sorted().toList()) {
                    if (Files.isSymbolicLink(path)) throw new IllegalArgumentException("Symlink in Ant class output");
                    if (!path.toString().endsWith(".class") || !Files.isRegularFile(path)) continue;
                    total += Files.size(path);
                    if (total > 128 * 1024 * 1024) throw new IllegalArgumentException("Ant class output exceeds limit");
                    analyzer.analyzeAll(path.toFile());
                }
            }
        }
        if (coverage.getClasses().isEmpty()) throw new IllegalArgumentException("No application classes matched conventional Ant source ownership");
        if (!coverage.getNoMatchClasses().isEmpty()) throw new IllegalArgumentException("Ant classes differ from measured execution bytes");
        try (var stream = Files.newOutputStream(output, StandardOpenOption.CREATE_NEW)) {
            var visitor = new XMLFormatter().createVisitor(stream);
            visitor.visitInfo(loader.getSessionInfoStore().getInfos(), loader.getExecutionDataStore().getContents());
            visitor.visitBundle(coverage.getBundle("Ant application src"), new DirectorySourceFileLocator(root.resolve("src").toFile(), "UTF-8", 4));
            visitor.visitEnd();
        }
    }
}
