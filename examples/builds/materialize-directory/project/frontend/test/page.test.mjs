import {test} from "node:test";
import assert from "node:assert/strict";
import {page} from "../src/page.mjs";
test("static page",()=>assert.match(page,/<h1>Hello, Oyzu!<\/h1>/));
