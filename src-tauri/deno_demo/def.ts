import Context from "./context.ts";
import "./event.ts";

//打包的时候需要把替换成./resource/"

let ctx = new Context();
ctx.start();
dispatchEvent(new CustomEvent("started"));
globalThis.applicationContext = ctx;