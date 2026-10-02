package example;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class GreetingTest {
  @Test
  void greets() {
    assertEquals("Hello, Oyzu!", Greeting.message());
  }
}
