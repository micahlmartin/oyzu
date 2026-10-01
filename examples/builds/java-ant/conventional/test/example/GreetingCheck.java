package example;
public final class GreetingCheck { public static void main(String[] args) { if (!"Hello, Oyzu!".equals(Greeting.message())) throw new AssertionError("wrong greeting"); } }
