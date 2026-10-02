package math

// Double scales positive values and clamps negative inputs.
func Double(value int) int {
	if value < 0 {
		return 0
	}
	return value * 2
}
