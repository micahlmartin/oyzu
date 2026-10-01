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
