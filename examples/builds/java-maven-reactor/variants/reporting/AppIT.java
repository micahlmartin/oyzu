package example;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import org.junit.jupiter.api.Test;

class AppIT {
  @Test
  void greets() {
    var output = new ByteArrayOutputStream();
    var previous = System.out;
    try (var stream = new PrintStream(output)) {
      System.setOut(stream);
      App.main(new String[0]);
    } finally {
      System.setOut(previous);
    }
    assertEquals("Hello, Oyzu!", output.toString().strip());
  }
}
