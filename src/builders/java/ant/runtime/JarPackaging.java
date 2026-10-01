import java.io.ByteArrayOutputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.time.LocalDateTime;
import java.util.Locale;
import java.util.TreeMap;
import java.util.jar.Attributes;
import java.util.jar.JarEntry;
import java.util.jar.JarFile;
import java.util.jar.JarOutputStream;
import java.util.jar.Manifest;

/** Preserve native JAR payloads and manifest sections, project the version, and
 * normalize archive timestamps. Signing belongs to a later publication step. */
class JarPackaging {
    public static void main(String[] args) throws Exception {
        if (args.length < 3 || args.length % 2 != 1) throw new IllegalArgumentException("expected version and source/destination pairs");
        for (int i = 1; i < args.length; i += 2) pack(Path.of(args[i]), Path.of(args[i + 1]), args[0]);
    }
    private static JarEntry entry(String name) {
        var entry = new JarEntry(name);
        entry.setTimeLocal(LocalDateTime.of(1980, 1, 1, 0, 0));
        return entry;
    }
    private static void pack(Path source, Path destination, String version) throws Exception {
        try (var input = new JarFile(source.toFile(), false)) {
            var entries = new TreeMap<String, JarEntry>();
            long size = 0;
            var iterator = input.entries();
            while (iterator.hasMoreElements()) {
                var item = iterator.nextElement();
                String name = item.getName();
                String upper = name.toUpperCase(Locale.ROOT);
                if (name.startsWith("/") || name.contains("\\") || name.contains("../") || name.equals("..")) throw new IllegalArgumentException("unsafe JAR entry");
                if (upper.startsWith("META-INF/") && (upper.endsWith(".SF") || upper.endsWith(".RSA") || upper.endsWith(".DSA") || upper.endsWith(".EC"))) throw new IllegalArgumentException("cannot version an already signed JAR");
                if (entries.put(name, item) != null) throw new IllegalArgumentException("duplicate JAR entry");
                size += item.getSize();
                if (entries.size() > 100000 || size > 256L * 1024 * 1024) throw new IllegalArgumentException("JAR exceeds packaging limit");
            }
            Manifest manifest = input.getManifest();
            if (manifest == null) manifest = new Manifest();
            manifest.getMainAttributes().put(Attributes.Name.MANIFEST_VERSION, "1.0");
            manifest.getMainAttributes().put(Attributes.Name.IMPLEMENTATION_VERSION, version);
            var buffer = new ByteArrayOutputStream();
            manifest.write(buffer);
            try (var output = new JarOutputStream(Files.newOutputStream(destination, StandardOpenOption.CREATE_NEW))) {
                output.putNextEntry(entry("META-INF/MANIFEST.MF"));
                output.write(buffer.toByteArray());
                output.closeEntry();
                for (var item : entries.values()) {
                    if (item.getName().equalsIgnoreCase("META-INF/MANIFEST.MF")) continue;
                    output.putNextEntry(entry(item.getName()));
                    try (var content = input.getInputStream(item)) { content.transferTo(output); }
                    output.closeEntry();
                }
            }
        }
    }
}
