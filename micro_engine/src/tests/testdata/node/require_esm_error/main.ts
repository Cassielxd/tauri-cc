import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

require("./esm.js");
