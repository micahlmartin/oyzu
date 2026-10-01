import {mkdirSync,writeFileSync} from "node:fs";
import {page} from "./src/page.mjs";
mkdirSync("dist",{recursive:true});
writeFileSync("dist/index.html",page);
