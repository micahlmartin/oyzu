package math

/*
static int add(int a, int b) { return a + b; }
*/
import "C"

func Add(a, b int) int { return int(C.add(C.int(a), C.int(b))) }
