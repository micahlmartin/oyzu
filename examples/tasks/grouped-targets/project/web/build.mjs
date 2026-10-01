import { mkdirSync, copyFileSync } from "node:fs";
mkdirSync("dist", { recursive: true });
copyFileSync("src/greeting.mjs", "dist/greeting.mjs");
