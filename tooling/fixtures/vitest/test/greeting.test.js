import { expect, test } from "vitest";
import { greet } from "../src/greeting.js";

test("greeting <&>", () => expect(greet("Oyzu")).toBe("Hello, Oyzu!"));
test.skip("optional case", () => {});
test.todo("future case");
