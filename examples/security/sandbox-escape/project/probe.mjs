import {readFileSync} from "node:fs";
const [mode,target]=process.argv.slice(2);
if (mode==="file") { console.log(readFileSync(target,"utf8")); }
else if (mode==="network") {
 const url=new URL(target);
 if(url.hostname!=="127.0.0.1") throw new Error("fixture accepts loopback only");
 const response=await fetch(url); console.log(await response.text());
} else throw new Error("use file <temporary sentinel> or network <loopback URL>");
