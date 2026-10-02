import java.nio.file.*;
import java.util.*;
import org.apache.tools.ant.*;
import org.apache.tools.ant.taskdefs.Java;
import org.apache.tools.ant.taskdefs.Javac;

/** Native Ant execution with explicit Java assertion outcomes and coverage. */
class AntTesting implements BuildListener {
    record Check(String name, Throwable failure) {}
    final List<Check> checks = new ArrayList<>();
    final Map<Task, String> running = new IdentityHashMap<>();
    final Set<Path> classes = new TreeSet<>();
    final Path root, data, agent;
    final String target;
    int junitTasks;

    AntTesting(Path root, Path data, Path agent, String target) {
        this.root = root;
        this.data = data;
        this.agent = agent;
        this.target = target;
    }

    public void taskStarted(BuildEvent event) {
        Task task = event.getTask();
        if (!(task instanceof UnknownElement element)) return;
        if (!Set.of("java", "javac", "junit").contains(element.getTaskName())) return;
        element.maybeConfigure();
        Object nativeTask = element.getRealThing();
        if (element.getTaskName().equals("junit")) {
            if (!(nativeTask instanceof AntJUnit)) throw new BuildException("Custom Ant junit task requires an explicit reporting adapter");
            junitTasks++;
        } else if (nativeTask instanceof Javac compiler) {
            for (String source : compiler.getSrcdir().list()) {
                if (Path.of(source).toAbsolutePath().normalize().equals(root.resolve("src"))) {
                    if (compiler.getDestdir() == null) throw new BuildException("Ant coverage requires an explicit javac destdir");
                    Path destination = compiler.getDestdir().toPath().toAbsolutePath().normalize();
                    if (!destination.startsWith(root)) throw new BuildException("Ant classes escape the captured project");
                    classes.add(destination);
                }
            }
        } else if (nativeTask instanceof Java launch) {
            // A code generator in a compile prerequisite is not a test case.
            if (task.getOwningTarget() == null || !task.getOwningTarget().getName().equals(target)) return;
            var attributes = element.getWrapper().getAttributeMap();
            for (String required : List.of("fork", "failonerror")) {
                String value = event.getProject().replaceProperties(String.valueOf(attributes.get(required)));
                if (!Project.toBoolean(value)) throw new BuildException("Ant Java assertion reporting requires " + required + "=true");
            }
            String name = event.getProject().replaceProperties(String.valueOf(attributes.get("classname")));
            if (name.equals("null")) throw new BuildException("Ant Java assertion reporting requires a classname");
            launch.createJvmarg().setValue("-javaagent:" + agent + "=destfile=" + data + ",append=true,output=file,dumponexit=true,jmx=false");
            running.put(task, name);
        }
    }

    public void taskFinished(BuildEvent event) {
        String name = running.remove(event.getTask());
        if (name != null) checks.add(new Check(name, event.getException()));
    }
    public void buildStarted(BuildEvent event) {}
    public void buildFinished(BuildEvent event) {}
    public void targetStarted(BuildEvent event) {}
    public void targetFinished(BuildEvent event) {}
    public void messageLogged(BuildEvent event) {}

    static String safe(String value) {
        StringBuilder result = new StringBuilder();
        value.codePoints().limit(8192).forEach(c -> result.appendCodePoint(
            c == 9 || c == 10 || c == 13 || c >= 32 && c != 0xfffe && c != 0xffff ? c : 0xfffd));
        return result.toString();
    }

    public static void main(String[] args) throws Exception {
        // Explicit paths allow the same adapter probe to run on all hosts.
        Path root = Path.of(".").toRealPath();
        Path junit = Path.of(args[0]).toAbsolutePath();
        Path coverage = Path.of(args[1]).toAbsolutePath();
        Path agent = Path.of(args[2]).toRealPath();
        // Refuse stale evidence; the build executor provides fresh output paths.
        if (Files.exists(junit, LinkOption.NOFOLLOW_LINKS) || Files.exists(coverage, LinkOption.NOFOLLOW_LINKS))
            throw new IllegalArgumentException("Ant report output already exists");
        Files.createDirectories(junit.getParent());
        Files.createDirectories(coverage.getParent());
        Path temporary = Files.createTempDirectory("oyzu-ant-coverage-");
        Path data = temporary.resolve("coverage.exec");
        Path nativeReports = Files.createDirectory(temporary.resolve("native-junit"));
        List<org.w3c.dom.Element> suites = List.of();
        var listener = new AntTesting(root, data, agent, args[4]);
        Throwable failure = null;
        try {
            var project = new Project();
            project.init();
            String antHome = System.getenv("ANT_HOME");
            if (antHome != null) project.setUserProperty("ant.home", antHome);
            project.addReference(AntJUnit.CONTEXT, new AntJUnit.Context(agent, data, nativeReports));
            project.addTaskDefinition("junit", AntJUnit.class);
            // Captured builds project a snapshot; direct tests retain the native
            // project's version properties and do not perform publication.
            if (!args[3].isEmpty()) {
                project.setUserProperty("version", args[3]);
                project.setUserProperty("oyzu.version", args[3]);
            }
            var logger = new DefaultLogger();
            logger.setOutputPrintStream(System.out);
            logger.setErrorPrintStream(System.err);
            logger.setMessageOutputLevel(Project.MSG_INFO);
            project.addBuildListener(logger);
            ProjectHelper.configureProject(project, root.resolve("build.xml").toFile());
            project.addBuildListener(listener);
            project.executeTarget(args[4]);
            if (listener.checks.isEmpty() && listener.junitTasks == 0) throw new BuildException("No supported Java assertion programs ran; configure native framework reports for this Ant target");
        } catch (Exception error) {
            failure = error;
        }
        try {
            suites = AntReports.read(nativeReports);
            if (AntReports.failed(suites) && failure == null) failure = new BuildException("Native JUnit tests failed");
            if (listener.junitTasks > 0 && suites.isEmpty() && failure == null) failure = new BuildException("Native JUnit produced no suites; no-test outcome integration is required");
        } catch (Exception error) {
            if (failure == null) failure = error;
            else failure.addSuppressed(error);
        }
        try {
            if (Files.exists(data)) AntCoverage.write(root, data, listener.classes, coverage);
            else if (failure == null) throw new IllegalStateException("Ant tests did not produce JaCoCo execution data");
        } catch (Exception error) {
            if (failure == null) failure = error;
            else failure.addSuppressed(error);
        }
        AntReports.write(junit, listener.checks, failure, suites);
        try (var paths = Files.walk(temporary)) {
            for (Path path : paths.sorted(Comparator.reverseOrder()).toList()) Files.delete(path);
        }
        if (failure != null) {
            failure.printStackTrace(System.err);
            System.exit(1);
        }
    }
}
