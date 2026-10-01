import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
mkdirSync(".events", {recursive:true});
const phase=process.argv[2];
appendFileSync(".events/order.txt", phase+"\n");
if (phase === "pre") {
  mkdirSync("generated", {recursive: true});
  writeFileSync("generated/input.json", JSON.stringify({message: "prepared greeting"}));
}
if (process.env.FAILURE_POINT===phase) process.exitCode=3;
