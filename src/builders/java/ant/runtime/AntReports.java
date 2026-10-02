import java.nio.file.*;
import java.util.*;
import javax.xml.XMLConstants;
import javax.xml.parsers.DocumentBuilderFactory;
import javax.xml.transform.TransformerFactory;
import javax.xml.transform.dom.DOMSource;
import javax.xml.transform.stream.StreamResult;
import org.w3c.dom.*;

/** Bounded native JUnit aggregation; native test cases retain their full details. */
class AntReports {
    static final long LIMIT = 16 * 1024 * 1024;

    static List<Element> read(Path directory) throws Exception {
        var factory = DocumentBuilderFactory.newInstance();
        factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true);
        factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_DTD, "");
        factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_SCHEMA, "");
        var parser = factory.newDocumentBuilder();
        var suites = new ArrayList<Element>();
        long total = 0;
        try (var paths = Files.walk(directory)) {
            for (Path path : paths.sorted().toList()) {
                if (Files.isSymbolicLink(path)) throw new IllegalArgumentException("Symlink in native JUnit reports");
                if (!Files.isRegularFile(path)) continue;
                total += Files.size(path);
                if (total > LIMIT) throw new IllegalArgumentException("Native Ant JUnit reports exceed limit");
                Element suite = parser.parse(path.toFile()).getDocumentElement();
                if (!suite.getTagName().equals("testsuite")) throw new IllegalArgumentException("Invalid native JUnit suite");
                suites.add(suite);
            }
        }
        return suites;
    }

    static boolean failed(List<Element> suites) {
        return suites.stream().anyMatch(s -> s.getElementsByTagName("failure").getLength() > 0 || s.getElementsByTagName("error").getLength() > 0);
    }

    static void write(Path output, List<AntTesting.Check> checks, Throwable error, List<Element> suites) throws Exception {
        var document = DocumentBuilderFactory.newInstance().newDocumentBuilder().newDocument();
        Element root = document.createElement("testsuites");
        document.appendChild(root);
        for (Element suite : suites) root.appendChild(document.importNode(suite, true));
        Element assertions = document.createElement("testsuite");
        assertions.setAttribute("name", "oyzu.ant.java-assertions");
        for (var check : checks) {
            Element test = document.createElement("testcase");
            test.setAttribute("name", AntTesting.safe(check.name()));
            if (check.failure() != null) {
                Element failure = document.createElement("failure");
                failure.setTextContent(AntTesting.safe(check.failure().toString()));
                test.appendChild(failure);
            }
            assertions.appendChild(test);
        }
        if (error != null && !failed(suites) && checks.stream().noneMatch(c -> c.failure() != null)) {
            Element test = document.createElement("testcase");
            test.setAttribute("name", "Ant test execution/reporting");
            Element failure = document.createElement("error");
            failure.setTextContent(AntTesting.safe(error.toString()));
            test.appendChild(failure);
            assertions.appendChild(test);
        }
        if (assertions.hasChildNodes() || suites.isEmpty()) root.appendChild(assertions);
        var factory = TransformerFactory.newInstance();
        factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_DTD, "");
        factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_STYLESHEET, "");
        var bytes = new java.io.ByteArrayOutputStream();
        factory.newTransformer().transform(new DOMSource(document), new StreamResult(bytes));
        if (bytes.size() > LIMIT) throw new IllegalArgumentException("Combined Ant JUnit report exceeds limit");
        Files.write(output, bytes.toByteArray(), StandardOpenOption.CREATE_NEW);
    }
}
