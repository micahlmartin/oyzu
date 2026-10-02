import { appendFileSync, mkdirSync } from "node:fs";
mkdirSync(".events", { recursive: true });
const phase = process.argv[2];
appendFileSync(".events/order.txt", phase + "\n");
if (process.env.FAILURE_POINT === phase) process.exitCode = 3;
