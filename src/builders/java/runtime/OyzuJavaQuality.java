// Owned native-tool adapter. Never downloads dependencies or executes project code.
import java.io.IOException;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.*;

class OyzuJavaQuality {
  private static final Set<String> EXCLUDED =
      Set.of(
          ".git",
          ".oyzu",
          ".oyzu-build",
          ".oyzu-maven",
          ".gradle",
          ".idea",
          "target",
          "build",
          "dist",
          "out",
          "node_modules",
          ".venv");

  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !Set.of("lint", "format-check", "format").contains(args[0])) {
      throw new IllegalArgumentException(
          "Expected one Java quality operation; use a task override for native options");
    }
    String configured = System.getenv("OYZU_JAVA_QUALITY_HOME");
    if (configured == null || !Path.of(configured).isAbsolute()) {
      throw new IllegalArgumentException(
          "Set absolute OYZU_JAVA_QUALITY_HOME to explicitly provisioned tools");
    }
    Path home = Path.of(configured);
    boolean lint = args[0].equals("lint");
    Path tool = home.resolve(lint ? "checkstyle.jar" : "google-java-format.jar");
    if (!Files.isRegularFile(tool)) throw new IOException("Missing provisioned tool: " + tool);
    Path root = Path.of("").toAbsolutePath().normalize();
    List<String> files = new ArrayList<>();
    Files.walkFileTree(
        root,
        new SimpleFileVisitor<Path>() {
          int entries;

          private void count() throws IOException {
            if (++entries > 200_000)
              throw new IOException("Java quality inventory exceeds 200000 entries");
          }

          @Override
          public FileVisitResult preVisitDirectory(Path path, BasicFileAttributes attrs)
              throws IOException {
            count();
            return !path.equals(root) && EXCLUDED.contains(path.getFileName().toString())
                ? FileVisitResult.SKIP_SUBTREE
                : FileVisitResult.CONTINUE;
          }

          @Override
          public FileVisitResult visitFile(Path path, BasicFileAttributes attrs)
              throws IOException {
            count();
            if (EXCLUDED.contains(path.getFileName().toString())) return FileVisitResult.CONTINUE;
            if (attrs.isSymbolicLink())
              throw new IOException("Java quality does not follow symbolic links: " + path);
            if (path.toString().endsWith(".java")) {
              if (!attrs.isRegularFile() || attrs.size() > 16 * 1024 * 1024)
                throw new IOException("Invalid Java source: " + path);
              files.add(path.toString());
            }
            return FileVisitResult.CONTINUE;
          }
        });
    Collections.sort(files);
    List<String> command =
        new ArrayList<>(
            List.of(
                Path.of(
                        System.getProperty("java.home"),
                        "bin",
                        System.getProperty("os.name").startsWith("Windows") ? "java.exe" : "java")
                    .toString(),
                "-jar",
                tool.toString()));
    if (lint) {
      Path config = home.resolve("checks.xml");
      if (!Files.isRegularFile(config))
        throw new IOException("Missing provisioned Java lint rules: " + config);
      command.addAll(List.of("-c", config.toString()));
    } else if (args[0].equals("format")) {
      command.add("--replace");
    } else {
      command.addAll(List.of("--dry-run", "--set-exit-if-changed"));
    }
    // Bound native command lines on Windows too; absolute paths cannot become options.
    int status = 0;
    List<String> batch = new ArrayList<>(command);
    int baseSize = command.stream().mapToInt(value -> value.length() * 2 + 4).sum();
    int size = baseSize;
    for (String file : files) {
      int length = file.length() * 2 + 4;
      if (baseSize + length > 16000)
        throw new IOException("Java quality path exceeds host argument limit");
      if (size + length > 16000) {
        if (run(batch) != 0) status = 1;
        batch = new ArrayList<>(command);
        size = baseSize;
      }
      batch.add(file);
      size += length;
    }
    if (batch.size() > command.size() && run(batch) != 0) status = 1;
    System.out.println(
        "Java "
            + args[0]
            + ": "
            + files.size()
            + " source candidates"
            + (files.isEmpty() ? " (not applicable)" : ""));
    if (status != 0) System.exit(status);
  }

  private static int run(List<String> command) throws Exception {
    Process process = new ProcessBuilder(command).inheritIO().start();
    try {
      return process.waitFor();
    } finally {
      if (process.isAlive()) process.destroyForcibly();
    }
  }
}
