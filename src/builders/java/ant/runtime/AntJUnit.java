import java.io.File;
import java.nio.file.*;
import java.util.IdentityHashMap;
import org.apache.tools.ant.BuildException;
import org.apache.tools.ant.taskdefs.optional.junit.*;

/** Extend native Ant execution only at its formatter and JVM argument boundaries. */
public class AntJUnit extends JUnitTask {
    static final String CONTEXT = "oyzu.ant.junit.context";
    record Context(Path agent, Path data, Path reports) {}
    private final IdentityHashMap<JUnitTest, File> outputs = new IdentityHashMap<>();
    private Path reports;
    private boolean installed;
    private static final String EXTENSION = ".oyzu-junit.xml";

    public AntJUnit() throws Exception { super(); }

    @Override public void execute() throws BuildException {
        Context context = getProject().getReference(CONTEXT);
        if (context == null) throw new BuildException("Missing captured Ant JUnit reporting context");
        // Preserve configured batching, selectors, conditions and halt behavior.
        // In-process JUnit needs a different coverage lifecycle; never silently fork it.
        var tests = allTests();
        while (tests.hasMoreElements()) {
            if (!tests.nextElement().getFork())
                throw new BuildException("Ant JUnit coverage currently requires fork=true");
        }
        try {
            reports = Files.createTempDirectory(context.reports(), "suite-");
        } catch (Exception error) { throw new BuildException(error); }
        outputs.clear();
        if (!installed) {
            var formatter = new FormatterElement();
            formatter.setClassname(FormatterElement.XML_FORMATTER_CLASS_NAME);
            formatter.setExtension(EXTENSION);
            formatter.setUseFile(true);
            addFormatter(formatter);
            createJvmarg().setValue("-javaagent:" + context.agent() + "=destfile=" + context.data() + ",append=true,output=file,dumponexit=true,jmx=false");
            installed = true;
        }
        super.execute();
    }

    @Override protected File getOutput(FormatterElement formatter, JUnitTest test) {
        if (!EXTENSION.equals(formatter.getExtension())) return super.getOutput(formatter, test);
        return outputs.computeIfAbsent(test, ignored -> reports.resolve("test-" + outputs.size() + ".xml").toFile());
    }
}
