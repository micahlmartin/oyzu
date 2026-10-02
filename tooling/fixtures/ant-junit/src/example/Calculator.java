package example;

public final class Calculator {
    public static int sign(int value) {
        if (value > 0) return 1;
        if (value < 0) return -1;
        return 0;
    }
}
