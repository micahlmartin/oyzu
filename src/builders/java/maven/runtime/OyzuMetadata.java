package dev.oyzu.maven;

import java.nio.file.Files;
import java.nio.file.Path;
import javax.xml.stream.XMLOutputFactory;
import javax.xml.stream.XMLStreamWriter;
import org.apache.maven.AbstractMavenLifecycleParticipant;
import org.apache.maven.MavenExecutionException;
import org.apache.maven.execution.MavenSession;
import org.apache.maven.model.Plugin;
import org.apache.maven.project.MavenProject;
import org.apache.maven.lifecycle.internal.LifecycleExecutionPlanCalculator;
import org.apache.maven.lifecycle.internal.LifecycleTask;
import org.apache.maven.plugin.PluginParameterExpressionEvaluator;

/** Native reactor metadata, evaluated inside Oyzu's constrained preparation. */
public final class OyzuMetadata extends AbstractMavenLifecycleParticipant {
    @Override
    public void afterProjectsRead(MavenSession session) throws MavenExecutionException {
        String destination = System.getenv("OYZU_MAVEN_METADATA");
        if (destination == null) return;
        try {
            Path root = Path.of(System.getenv("OYZU_MAVEN_WORKSPACE")).toRealPath();
            try (var output = Files.newOutputStream(Path.of(destination))) {
                XMLStreamWriter xml = XMLOutputFactory.newFactory().createXMLStreamWriter(output, "UTF-8");
                xml.writeStartDocument("UTF-8", "1.0");
                xml.writeStartElement("reactor");
                for (MavenProject project : session.getProjects()) {
                    xml.writeStartElement("project");
                    value(xml, "groupId", project.getGroupId());
                    value(xml, "artifactId", project.getArtifactId());
                    value(xml, "version", project.getVersion());
                    value(xml, "packaging", project.getPackaging());
                    value(xml, "path", relative(root, project.getBasedir().toPath()));
                    value(xml, "pom", relative(root, project.getFile().toPath()));
                    value(xml, "buildDirectory", relative(root, Path.of(project.getBuild().getDirectory())));
                    value(xml, "finalName", project.getBuild().getFinalName());
                    xml.writeStartElement("testRoots");
                    for (String source : project.getTestCompileSourceRoots()) {
                        value(xml, "path", relative(root, Path.of(source)));
                    }
                    xml.writeEndElement();
                    if ("true".equals(System.getenv("OYZU_MAVEN_TEST_PLAN"))) {
                        testReports(xml, root, session, project);
                    }
                    xml.writeStartElement("dependencies");
                    for (var dependency : project.getDependencies()) {
                        xml.writeStartElement("dependency");
                        value(xml, "groupId", dependency.getGroupId());
                        value(xml, "artifactId", dependency.getArtifactId());
                        value(xml, "version", dependency.getVersion());
                        value(xml, "scope", dependency.getScope());
                        value(xml, "type", dependency.getType());
                        if (dependency.getSystemPath() != null) {
                            value(xml, "systemPath", relative(root, Path.of(dependency.getSystemPath())));
                        }
                        xml.writeEndElement();
                    }
                    xml.writeEndElement();
                    xml.writeStartElement("plugins");
                    for (Plugin plugin : project.getBuildPlugins()) {
                        xml.writeStartElement("plugin");
                        value(xml, "groupId", plugin.getGroupId());
                        value(xml, "artifactId", plugin.getArtifactId());
                        value(xml, "version", plugin.getVersion());
                        xml.writeEndElement();
                    }
                    xml.writeEndElement();
                    xml.writeEndElement();
                }
                xml.writeEndElement();
                xml.writeEndDocument();
                xml.close();
            }
        } catch (Exception error) {
            throw new MavenExecutionException("Oyzu cannot capture contained Maven reactor metadata", error);
        }
    }

    private static void testReports(XMLStreamWriter xml, Path root, MavenSession session,
                                    MavenProject project) throws Exception {
        MavenSession scoped = session.clone();
        scoped.setCurrentProject(project);
        var calculator = session.getContainer().lookup(LifecycleExecutionPlanCalculator.class);
        var plan = calculator.calculateExecutionPlan(scoped, project,
                java.util.List.of(new LifecycleTask("verify")), true);
        xml.writeStartElement("testReports");
        var directories = new java.util.TreeSet<String>();
        for (var execution : plan.getMojoExecutions()) {
            String plugin = execution.getArtifactId();
            if (!"org.apache.maven.plugins".equals(execution.getGroupId())
                    || !("maven-surefire-plugin".equals(plugin) && "test".equals(execution.getGoal())
                    || "maven-failsafe-plugin".equals(plugin) && "integration-test".equals(execution.getGoal()))) {
                continue;
            }
            var parameter = execution.getConfiguration().getChild("reportsDirectory");
            if (parameter == null) throw new IllegalArgumentException("Missing native test report directory");
            String expression = parameter.getValue();
            if (expression == null) expression = parameter.getAttribute("default-value");
            var evaluator = new PluginParameterExpressionEvaluator(scoped, execution);
            Object value = evaluator.evaluate(expression);
            if (value == null && parameter.getAttribute("default-value") != null) {
                value = evaluator.evaluate(parameter.getAttribute("default-value"));
            }
            if (value == null || value.toString().contains("${")) {
                throw new IllegalArgumentException("Unresolved native test report directory");
            }
            var directory = evaluator.alignToBaseDirectory(new java.io.File(value.toString()));
            directories.add(relative(root, directory.toPath()));
        }
        for (String directory : directories) value(xml, "directory", directory);
        xml.writeEndElement();
    }

    private static String relative(Path root, Path path) {
        Path normalized = path.toAbsolutePath().normalize();
        if (!normalized.startsWith(root)) {
            throw new IllegalArgumentException("Maven path is outside captured source: " + path);
        }
        String relative = root.relativize(normalized).toString().replace(java.io.File.separatorChar, '/');
        return relative.isEmpty() ? "." : relative;
    }

    private static void value(XMLStreamWriter xml, String name, String value) throws Exception {
        xml.writeStartElement(name);
        xml.writeCharacters(value == null ? "" : value);
        xml.writeEndElement();
    }
}
