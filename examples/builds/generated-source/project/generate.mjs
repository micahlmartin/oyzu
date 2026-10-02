import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
const { greeting } = JSON.parse(readFileSync("schema.json", "utf8"));
mkdirSync("generated", { recursive: true });
writeFileSync(
  "generated/message.mjs",
  "export const message = " + JSON.stringify(greeting) + ";\n",
);
