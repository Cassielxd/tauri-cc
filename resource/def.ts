import Context from "./context.ts";
import "./event.ts";

await import("./db.ts");
let ctx = new Context("./resource/");
ctx.start();
dispatchEvent(new CustomEvent("started"));
globalThis.applicationContext = ctx;