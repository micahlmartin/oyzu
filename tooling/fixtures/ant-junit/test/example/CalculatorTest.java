package example;

import static org.junit.Assert.assertEquals;

import org.junit.Ignore;
import org.junit.Test;

public class CalculatorTest {
  @Test
  public void positive() {
    assertEquals(1, Calculator.sign(2));
  }

  @Test
  public void zero() {
    assertEquals(0, Calculator.sign(0));
  }

  @Ignore("Native skip evidence")
  @Test
  public void negative() {
    assertEquals(-1, Calculator.sign(-1));
  }
}
