import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.TreeMap;
import javax.xml.stream.XMLOutputFactory;
import org.apache.tools.ant.Project;
import org.apache.tools.ant.ProjectHelper;
import org.apache.tools.ant.UnknownElement;

/** Evaluate Ant metadata inside the denied-network preparation executor. */
class AntMetadata {
    public static void main(String[] args) throws Exception {
        var project = new Project();
        project.init();
        if (args.length == 3) {
            project.setUserProperty("version", args[2]);
            project.setUserProperty("oyzu.version", args[2]);
        }
        ProjectHelper.configureProject(project, new File(args[0]));
        String version = project.getProperty("version");
        if (version == null) version = project.getProperty("project.version");
        if (version == null) version = "0.0.0";
        try (var output = Files.newOutputStream(Path.of(args[1]))) {
            var xml = XMLOutputFactory.newFactory().createXMLStreamWriter(output, "UTF-8");
            xml.writeStartElement("project");
            xml.writeAttribute("version", version);
            xml.writeAttribute("antVersion", org.apache.tools.ant.Main.getAntVersion());
            for (var entry : new TreeMap<>(project.getTargets()).entrySet()) {
                if (entry.getKey().isEmpty()) continue;
                xml.writeStartElement("target");
                xml.writeAttribute("name", entry.getKey());
                for (var task : entry.getValue().getTasks()) {
                    if (task instanceof UnknownElement element && element.getTaskName().equals("jar")) {
                        Object raw = element.getWrapper().getAttributeMap().get("destfile");
                        if (raw == null) throw new IllegalArgumentException("Ant jar has no destfile");
                        String path = project.replaceProperties(raw.toString());
                        if (path.contains("${")) throw new IllegalArgumentException("Ant jar destination depends on runtime properties: " + path);
                        xml.writeEmptyElement("jar");
                        xml.writeAttribute("path", project.resolveFile(path).toPath().normalize().toString().replace(File.separatorChar, '/'));
                    }
                }
                xml.writeEndElement();
            }
            xml.writeEndElement();
            xml.close();
        }
    }
}
